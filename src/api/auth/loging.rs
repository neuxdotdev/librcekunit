//! Login client for authenticating with the CekUnit application.
//!
//! This module provides the [`LoginClient`] struct, which handles the entire login process:
//! - Fetching a CSRF token from the login page.
//! - Submitting credentials (email/password) along with the token.
//! - Extracting session cookies from the response.
//! - Persisting the session (cookies and token) in a cache file.
//!
//! The client includes retry logic with exponential backoff for transient failures
//! and uses a configurable HTTP client with connection pooling and timeouts.

use crate::api::auth::utils::{
    cache::{CacheData, CacheManager, Cookie},
    cookies::{add_cookies_to_headers, extract_cookies},
    token::extract_csrf_token,
};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// User-Agent string used for all requests.
///
/// Mimics a recent Firefox browser to avoid being blocked by the server.
const USER_AGENT_STR: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0";

/// Timeout for individual HTTP requests (15 seconds).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum number of retry attempts for failed requests (CSRF fetch and login).
const MAX_RETRIES: u32 = 3;

/// Initial delay before the first retry (100 ms). Subsequent delays double.
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(100);

/// Client for performing login operations.
///
/// This struct holds an HTTP client, the environment configuration, and a cache manager.
/// It provides methods to log in, fetch CSRF tokens, and access cached sessions.
///
/// # Example
/// ```no_run
/// use cekunit_client::api::auth::LoginClient;
///
/// let mut client = LoginClient::new()?;
/// let session = client.login()?;
/// println!("Logged in, cookies: {}", session.cookies.len());
/// # Ok::<(), cekunit_client::handler::error::ApiError>(())
/// ```
pub struct LoginClient {
    /// The underlying reqwest blocking client.
    pub client: Client,
    /// Environment configuration loaded from variables.
    pub config: EnvConfig,
    /// Manager for reading/writing the session cache.
    pub cache_manager: CacheManager,
}

impl LoginClient {
    /// Creates a new `LoginClient` with default configuration loaded from environment variables.
    ///
    /// This is a convenience constructor that calls [`EnvConfig::load()`] and then
    /// [`Self::with_config`].
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - Environment variables are missing or invalid.
    /// - The cache directory cannot be created.
    /// - The HTTP client cannot be built.
    pub fn new() -> Result<Self, ApiError> {
        let config = EnvConfig::load()?;
        Self::with_config(config)
    }

    /// Creates a new `LoginClient` with a given configuration.
    ///
    /// This allows using a pre‑loaded configuration, for example when sharing
    /// configuration between multiple clients.
    ///
    /// # Arguments
    /// * `config` - The environment configuration to use.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - The cache directory cannot be created.
    /// - The HTTP client cannot be built.
    pub fn with_config(config: EnvConfig) -> Result<Self, ApiError> {
        let cache_manager = CacheManager::new()?;
        let client = Self::build_client()?;
        Ok(Self {
            client,
            config,
            cache_manager,
        })
    }

    /// Builds and configures the HTTP client.
    ///
    /// The client is configured with:
    /// - A custom User-Agent.
    /// - Automatic cookie storage (enabled).
    /// - A 15‑second timeout for requests.
    /// - Connection verbose logging (for debugging).
    /// - TCP keepalive (60 seconds).
    /// - Connection pool idle timeout (90 seconds).
    /// - Maximum 10 idle connections per host.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the client builder fails (e.g., invalid configuration).
    fn build_client() -> Result<Client, ApiError> {
        Client::builder()
            .user_agent(USER_AGENT_STR)
            .cookie_store(true)
            .timeout(REQUEST_TIMEOUT)
            .connection_verbose(true)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| {
                log::error!("Failed to build HTTP client: {}", e);
                ApiError::from(e)
            })
    }

    /// Performs the full login flow and returns the cached session data.
    ///
    /// The steps are:
    /// 1. Validate that credentials are present (non‑empty).
    /// 2. Fetch a CSRF token from the login page (with retries).
    /// 3. Build a login form with the token, email, and password.
    /// 4. Attach any previously cached cookies (if any).
    /// 5. Send a POST request to the login endpoint (with retries).
    /// 6. Validate the response status.
    /// 7. Extract cookies from the response headers.
    /// 8. Build a `CacheData` object containing cookies, CSRF token, and timestamp.
    /// 9. Save the cache data to the cache file.
    ///
    /// # Returns
    /// The newly created [`CacheData`] representing the authenticated session.
    ///
    /// # Errors
    /// Returns [`ApiError`] if any step fails:
    /// - Credentials are empty.
    /// - CSRF token cannot be fetched (after retries).
    /// - Login request fails (after retries).
    /// - Response status indicates failure (4xx or 5xx).
    /// - Response body cannot be read.
    /// - Cache cannot be saved.
    pub fn login(&mut self) -> Result<CacheData, ApiError> {
        log::info!(
            " Starting login process to: {}",
            self.config.full_login_url()
        );
        self.validate_credentials()?;

        let csrf_token = self.fetch_csrf_token_with_retry()?;
        log::debug!(
            " CSRF token fetched: {}…",
            &csrf_token[..10.min(csrf_token.len())]
        );

        let mut login_form = HashMap::new();
        login_form.insert("_token", csrf_token.as_str());
        login_form.insert("email", self.config.user_email.as_str());
        login_form.insert("password", self.config.user_password.as_str());

        let mut headers = self.build_base_headers()?;
        self.attach_cached_cookies(&mut headers)?;

        log::info!(" Sending login request...");
        let response = self.execute_login_request(&headers, &login_form)?;

        let status = response.status();
        let headers_clone = response.headers().clone();
        let body = response.text().map_err(|e| {
            log::error!("Failed to read response body: {}", e);
            ApiError::from(e)
        })?;

        self.validate_login_response(status, &body)?;

        let cookies = extract_cookies(&headers_clone);
        log::debug!(" Received {} cookies", cookies.len());
        if cookies.is_empty() {
            log::warn!("️ No cookies received from login response!");
        }

        let cache_data = self.build_cache_data(cookies, csrf_token)?;
        self.cache_manager.save(&cache_data)?;

        log::info!(
            " Login successful. Cache saved at {:?}",
            self.cache_manager.cache_file_path()
        );

        Ok(cache_data)
    }

    /// Fetches a CSRF token from the login page (single attempt, no retry).
    ///
    /// This method sends a GET request to the login URL, checks that the response
    /// is successful, reads the HTML body, and extracts the CSRF token using
    /// [`extract_csrf_token`].
    ///
    /// # Returns
    /// The extracted CSRF token string.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - The HTTP request fails (network, timeout).
    /// - The response status is not successful.
    /// - The response body cannot be read.
    /// - No CSRF token is found in the HTML.
    pub fn fetch_csrf_token(&self) -> Result<String, ApiError> {
        log::debug!(" Fetching CSRF token from login page");
        let response = self
            .client
            .get(self.config.full_login_url())
            .send()
            .map_err(|e| {
                log::error!("Network error while fetching CSRF token: {}", e);
                ApiError::from(e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body_snippet = response
                .text()
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect::<String>();
            log::error!(
                "Failed to fetch login page: HTTP {} - {}",
                status,
                body_snippet
            );
            return Err(ApiError::LoginFailed(format!(
                "Failed to fetch login page (HTTP {}): {}",
                status, body_snippet
            )));
        }

        let html = response.text().map_err(|e| {
            log::error!("Failed to read response body: {}", e);
            ApiError::from(e)
        })?;

        extract_csrf_token(&html).map_err(|e| {
            log::error!("CSRF token not found in login page HTML");
            e
        })
    }

    /// Returns the currently cached session, if any.
    ///
    /// This method simply delegates to [`CacheManager::load`].
    pub fn get_cached_session(&self) -> Result<Option<CacheData>, ApiError> {
        self.cache_manager.load()
    }

    /// Returns the path to the session cache file.
    pub fn cache_file_path(&self) -> std::path::PathBuf {
        self.cache_manager.cache_file_path().to_path_buf()
    }

    /// Returns a reference to the environment configuration.
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }

    /// Returns a reference to the cache manager.
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }

    /// Validates that the credentials in the configuration are not empty.
    ///
    /// Logs a warning if the email does not contain '@' (but does not fail).
    ///
    /// # Errors
    /// Returns [`ApiError::LoginFailed`] if `USER_EMAIL` or `USER_PASSWORD` is empty.
    fn validate_credentials(&self) -> Result<(), ApiError> {
        if self.config.user_email.is_empty() {
            log::error!(" USER_EMAIL is empty");
            return Err(ApiError::LoginFailed("USER_EMAIL cannot be empty".into()));
        }
        if self.config.user_password.is_empty() {
            log::error!(" USER_PASSWORD is empty");
            return Err(ApiError::LoginFailed(
                "USER_PASSWORD cannot be empty".into(),
            ));
        }
        if !self.config.user_email.contains('@') {
            log::warn!("️ USER_EMAIL does not contain '@', mungkin bukan format email");
        }
        Ok(())
    }

    /// Validates the login response status and body.
    ///
    /// If the status is successful (2xx), returns `Ok(())`.
    /// Otherwise, maps the status code to an appropriate [`ApiError`] variant.
    ///
    /// # Arguments
    /// * `status` - HTTP status code.
    /// * `body` - Response body (used for preview in error messages).
    fn validate_login_response(&self, status: StatusCode, body: &str) -> Result<(), ApiError> {
        if status.is_success() {
            return Ok(());
        }

        let clean_body = body.split('<').next().unwrap_or("Unknown error").trim();
        log::error!(" Login failed: HTTP {} - {}", status, clean_body);

        match status.as_u16() {
            419 => Err(ApiError::LoginFailed(
                "CSRF token expired or invalid".into(),
            )),
            422 => Err(ApiError::LoginFailed(
                "Validation error: email/password incorrect".into(),
            )),
            429 => Err(ApiError::LoginFailed(
                "Too many requests, please try later".into(),
            )),
            500..=599 => Err(ApiError::LoginFailed(format!(
                "Server error (HTTP {})",
                status
            ))),
            _ => Err(ApiError::LoginFailed(format!(
                "HTTP {}: {}",
                status, clean_body
            ))),
        }
    }

    /// Fetches a CSRF token with retry logic.
    ///
    /// Retries up to [`MAX_RETRIES`] times with exponential backoff.
    /// The first retry is delayed by [`INITIAL_RETRY_DELAY`], then doubled each attempt.
    ///
    /// # Returns
    /// The CSRF token if successful.
    ///
    /// # Errors
    /// Returns the last error encountered, or a generic error if all retries fail.
    fn fetch_csrf_token_with_retry(&self) -> Result<String, ApiError> {
        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            match self.fetch_csrf_token() {
                Ok(token) => return Ok(token),
                Err(e) => {
                    log::warn!("️ CSRF fetch attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        let delay = INITIAL_RETRY_DELAY * 2_u32.pow(attempt);
                        std::thread::sleep(delay);
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            ApiError::LoginFailed("Failed to fetch CSRF token after retries".into())
        }))
    }

    /// Executes the login POST request with retry logic.
    ///
    /// Retries up to [`MAX_RETRIES`] times with exponential backoff.
    /// Only retries on network errors or server errors (5xx). Client errors (4xx)
    /// are considered final and are not retried.
    ///
    /// # Arguments
    /// * `headers` - Headers to attach to the request.
    /// * `form` - Form data (including `_token`, `email`, `password`).
    ///
    /// # Returns
    /// The response on success (any 2xx or 3xx status, or any status <500 after retry?).
    /// The implementation returns the response if status is success or <500, meaning
    /// it will also return on 3xx redirects, which is acceptable.
    ///
    /// # Errors
    /// Returns the last error encountered, or a generic error if all retries fail.
    fn execute_login_request(
        &self,
        headers: &HeaderMap,
        form: &HashMap<&str, &str>,
    ) -> Result<reqwest::blocking::Response, ApiError> {
        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            match self
                .client
                .post(self.config.full_login_url())
                .headers(headers.clone())
                .form(form)
                .send()
            {
                Ok(response) => {
                    if response.status().is_success() || response.status().as_u16() < 500 {
                        return Ok(response);
                    }
                    log::warn!(
                        "️ Server error (HTTP {}), attempt {} will retry",
                        response.status(),
                        attempt + 1
                    );
                    last_error = Some(ApiError::RequestFailed(format!(
                        "HTTP {}",
                        response.status()
                    )));
                }
                Err(e) => {
                    log::warn!("️ Network error on attempt {}: {}", attempt + 1, e);
                    last_error = Some(ApiError::from(e));
                }
            }
            if attempt < MAX_RETRIES - 1 {
                let delay = INITIAL_RETRY_DELAY * 2_u32.pow(attempt);
                std::thread::sleep(delay);
            }
        }
        Err(last_error
            .unwrap_or_else(|| ApiError::LoginFailed("Login request failed after retries".into())))
    }

    /// Builds the base headers for the login request.
    ///
    /// Includes:
    /// - `User-Agent`
    /// - `Content-Type: application/x-www-form-urlencoded`
    fn build_base_headers(&self) -> Result<HeaderMap, ApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            USER_AGENT_STR
                .parse()
                .map_err(|e| ApiError::CacheError(format!("Invalid User-Agent header: {}", e)))?,
        );
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded"
                .parse()
                .map_err(|e| ApiError::CacheError(format!("Invalid Content-Type header: {}", e)))?,
        );
        Ok(headers)
    }

    /// Attaches any cached cookies to the provided headers.
    ///
    /// If a cached session exists, its cookies are loaded and added to the headers
    /// using [`add_cookies_to_headers`]. This is useful for maintaining session
    /// across requests (though for login we usually don't need previous cookies,
    /// but it's harmless).
    fn attach_cached_cookies(&self, headers: &mut HeaderMap) -> Result<(), ApiError> {
        if let Some(cache) = self.cache_manager.load()? {
            let cookie_map: HashMap<String, String> = cache
                .cookies
                .iter()
                .map(|c| (c.name.clone(), c.value.clone()))
                .collect();
            if !cookie_map.is_empty() {
                add_cookies_to_headers(headers, &cookie_map)?;
                log::debug!(" Loaded {} cached cookies", cookie_map.len());
            }
        }
        Ok(())
    }

    /// Builds a `CacheData` object from the received cookies and CSRF token.
    ///
    /// Converts the cookie map into a vector of [`Cookie`] structs, using the base URL
    /// as the domain and default path `/`. Also sets `http_only` to `true` and `secure`
    /// to `false` (these may be inaccurate but are not critical for reuse).
    ///
    /// The timestamp is set to the current time.
    fn build_cache_data(
        &self,
        cookies: HashMap<String, String>,
        csrf_token: String,
    ) -> Result<CacheData, ApiError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Ok(CacheData {
            cookies: cookies
                .into_iter()
                .map(|(name, value)| Cookie {
                    name,
                    value,
                    domain: self.config.base_url.clone(),
                    path: "/".to_string(),
                    http_only: true,
                    secure: false,
                })
                .collect(),
            csrf_token,
            logged_in: true,
            timestamp: now,
        })
    }
}
