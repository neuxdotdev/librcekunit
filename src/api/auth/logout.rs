//! Logout client for terminating a session with the CekUnit application.
//!
//! This module provides the [`LogoutClient`] struct, which handles the logout process:
//! - Loading the current session from cache.
//! - Sending a POST request to the logout endpoint with the CSRF token.
//! - Clearing the session cache upon successful logout.
//!
//! The client includes retry logic with exponential backoff for transient failures
//! and uses the same HTTP client configuration as the login client.

use crate::api::auth::utils::{
    cache::{CacheData, CacheManager},
    cookies::add_cookies_to_headers,
};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;

/// User-Agent string used for logout requests.
///
/// Matches the one used by the login client to appear as a consistent browser.
const USER_AGENT_STR: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0";

/// Timeout for individual HTTP requests (15 seconds).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum number of retry attempts for failed logout requests.
const MAX_RETRIES: u32 = 3;

/// Initial delay before the first retry (100 ms). Subsequent delays double.
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(100);

/// Client for performing logout operations.
///
/// This struct holds an HTTP client, the environment configuration, and a cache manager.
/// It provides methods to log out using either the cached CSRF token or a provided one,
/// as well as utilities to inspect and clear the cache.
///
/// # Example
/// ```no_run
/// use cekunit_client::api::auth::LogoutClient;
///
/// let mut client = LogoutClient::new()?;
/// client.logout()?; // uses cached token
/// # Ok::<(), cekunit_client::handler::error::ApiError>(())
/// ```
pub struct LogoutClient {
    /// The underlying reqwest blocking client.
    pub client: Client,
    /// Environment configuration loaded from variables.
    pub config: EnvConfig,
    /// Manager for reading/writing the session cache.
    pub cache_manager: CacheManager,
}

impl LogoutClient {
    /// Creates a new `LogoutClient` with default configuration loaded from environment variables.
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

    /// Creates a new `LogoutClient` with a given configuration.
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
    /// The client is configured identically to the login client to ensure
    /// consistent behaviour and cookie handling. Settings include:
    /// - Custom User-Agent.
    /// - Automatic cookie storage.
    /// - 15‑second timeout.
    /// - Connection verbose logging (for debugging).
    /// - TCP keepalive (60 seconds).
    /// - Connection pool idle timeout (90 seconds).
    /// - Maximum 10 idle connections per host.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the client builder fails.
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
                log::error!(" Failed to build HTTP client: {}", e);
                ApiError::from(e)
            })
    }

    /// Performs logout using the CSRF token stored in the cache.
    ///
    /// Steps:
    /// 1. Load a valid session from the cache (must be logged in).
    /// 2. Build headers including cookies and content type.
    /// 3. Send a POST request to the logout endpoint with the cached token.
    /// 4. On success (HTTP 2xx, 302, or 303), clear the cache.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists in the cache.
    /// - The logout request fails after retries.
    /// - The server returns a client error (4xx) that is not retried.
    pub fn logout(&mut self) -> Result<(), ApiError> {
        log::info!(" Starting logout process (using cached token)");
        let cache_data = self.load_valid_session()?;
        let headers = self.build_headers(&cache_data)?;
        let mut form = HashMap::new();
        form.insert("_token", cache_data.csrf_token.as_str());
        self.execute_logout_request(headers, form)
    }

    /// Performs logout using a provided CSRF token.
    ///
    /// This method is useful when a fresh token has been obtained from a dashboard page.
    /// Steps are identical to [`logout`](Self::logout), but the token is taken from the argument.
    ///
    /// # Arguments
    /// * `csrf_token` - A valid CSRF token (typically fetched from a page after login).
    ///
    /// # Errors
    /// Same as [`logout`](Self::logout).
    pub fn logout_with_token(&mut self, csrf_token: &str) -> Result<(), ApiError> {
        log::info!(" Starting logout process (using provided token)");
        let cache_data = self.load_valid_session()?;
        let headers = self.build_headers(&cache_data)?;
        let mut form = HashMap::new();
        form.insert("_token", csrf_token);
        self.execute_logout_request(headers, form)
    }

    /// Manually clears the session cache.
    ///
    /// This can be used to force a logout without sending a request to the server,
    /// or to clean up after a failed logout.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the cache file cannot be removed.
    pub fn clear_cache(&mut self) -> Result<(), ApiError> {
        log::info!(" Clearing cache manually");
        self.cache_manager.clear()
    }

    /// Loads the cached session data, if any.
    ///
    /// # Returns
    /// `Ok(Some(CacheData))` if a cache file exists and can be parsed.
    /// `Ok(None)` if no cache file exists.
    /// `Err(ApiError)` if the cache file exists but cannot be read or parsed.
    pub fn load_cache(&self) -> Result<Option<CacheData>, ApiError> {
        self.cache_manager.load()
    }

    /// Returns a reference to the environment configuration.
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }

    /// Returns a reference to the cache manager.
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }

    /// Loads a session that is marked as logged in.
    ///
    /// If the cache exists but `logged_in` is false, the cache is cleared and
    /// [`ApiError::NotAuthenticated`] is returned.
    /// If no cache exists, returns [`ApiError::NotAuthenticated`].
    ///
    /// # Errors
    /// - [`ApiError::NotAuthenticated`] if no valid logged‑in session is found.
    /// - [`ApiError::CacheError`] if loading or clearing the cache fails.
    fn load_valid_session(&self) -> Result<CacheData, ApiError> {
        match self.cache_manager.load()? {
            Some(data) if data.logged_in => {
                log::debug!(" Valid session loaded ({} cookies)", data.cookies.len());
                Ok(data)
            }
            Some(_) => {
                log::warn!("️ Session exists but not logged in – clearing cache");
                self.cache_manager.clear()?;
                Err(ApiError::NotAuthenticated)
            }
            None => {
                log::warn!("️ No active session found");
                Err(ApiError::NotAuthenticated)
            }
        }
    }

    /// Builds the headers for the logout request.
    ///
    /// Includes:
    /// - `User-Agent`
    /// - `Content-Type: application/x-www-form-urlencoded`
    /// - `Cookie` header built from the cached cookies.
    ///
    /// # Errors
    /// Returns [`ApiError::CacheError`] if header values are invalid (unlikely).
    fn build_headers(&self, cache_data: &CacheData) -> Result<HeaderMap, ApiError> {
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

        let cookie_map: HashMap<String, String> = cache_data
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect();

        if !cookie_map.is_empty() {
            add_cookies_to_headers(&mut headers, &cookie_map)?;
            log::debug!(" Attached {} cookies to logout request", cookie_map.len());
        }

        Ok(headers)
    }

    /// Executes the logout POST request with retry logic.
    ///
    /// Retries up to [`MAX_RETRIES`] times with exponential backoff.
    /// Only retries on network errors or server errors (5xx). Client errors (4xx)
    /// are considered final and are not retried.
    ///
    /// On success (HTTP 2xx, 302, or 303), the cache is cleared and `Ok(())` is returned.
    ///
    /// # Arguments
    /// * `headers` - Headers to attach to the request.
    /// * `form` - Form data containing the `_token`.
    ///
    /// # Errors
    /// Returns the last error encountered, or a mapped error from the response status.
    fn execute_logout_request(
        &mut self,
        headers: HeaderMap,
        form: HashMap<&str, &str>,
    ) -> Result<(), ApiError> {
        let url = self.config.full_logout_url();
        log::info!(" Sending logout request to: {}", url);

        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            match self
                .client
                .post(&url)
                .headers(headers.clone())
                .form(&form)
                .send()
            {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() || status.as_u16() == 302 || status.as_u16() == 303 {
                        log::debug!(" Logout response status: {}", status);
                        if let Err(e) = self.cache_manager.clear() {
                            log::error!(" Failed to clear cache after logout: {}", e);
                        } else {
                            log::info!(" Cache cleared successfully");
                        }
                        log::info!(" Logout successful!");
                        return Ok(());
                    }

                    let body = response.text().unwrap_or_default();
                    let clean_body = body.split('<').next().unwrap_or("Unknown error").trim();

                    if status.as_u16() < 500 {
                        log::error!(
                            " Logout failed (client error): HTTP {} - {}",
                            status,
                            clean_body
                        );
                        return Err(self.map_logout_error(status, clean_body));
                    }

                    log::warn!(
                        "️ Logout server error (HTTP {}), attempt {} will retry",
                        status,
                        attempt + 1
                    );
                    last_error = Some(ApiError::LogoutFailed(format!(
                        "HTTP {} - {}",
                        status, clean_body
                    )));
                }
                Err(e) => {
                    log::warn!("️ Logout network error on attempt {}: {}", attempt + 1, e);
                    last_error = Some(ApiError::from(e));
                }
            }

            if attempt < MAX_RETRIES - 1 {
                let delay = INITIAL_RETRY_DELAY * 2_u32.pow(attempt);
                log::debug!(" Waiting {:?} before retry...", delay);
                std::thread::sleep(delay);
            }
        }

        let err = last_error.unwrap_or_else(|| {
            ApiError::LogoutFailed("Logout request failed after maximum retries".into())
        });
        log::error!(" All logout retry attempts failed: {}", err);
        Err(err)
    }

    /// Maps an HTTP status code to a specific [`ApiError::LogoutFailed`] variant.
    ///
    /// Provides human‑readable messages for common status codes:
    /// - 419 → CSRF token expired
    /// - 422 → Validation error (missing token)
    /// - 429 → Too many requests
    /// - 5xx → Server error
    /// - Others → Generic message with status and body preview
    fn map_logout_error(&self, status: StatusCode, body: &str) -> ApiError {
        match status.as_u16() {
            419 => ApiError::LogoutFailed("CSRF token expired or invalid".into()),
            422 => ApiError::LogoutFailed("Validation error (maybe missing _token)".into()),
            429 => ApiError::LogoutFailed("Too many requests, please try later".into()),
            500..=599 => ApiError::LogoutFailed(format!("Server error (HTTP {})", status)),
            _ => ApiError::LogoutFailed(format!("HTTP {}: {}", status, body)),
        }
    }
}
