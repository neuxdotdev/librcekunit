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
const USER_AGENT_STR: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(100);
pub struct LoginClient {
    pub client: Client,
    pub config: EnvConfig,
    pub cache_manager: CacheManager,
}
impl LoginClient {
    pub fn new() -> Result<Self, ApiError> {
        let config = EnvConfig::load()?;
        Self::with_config(config)
    }
    pub fn with_config(config: EnvConfig) -> Result<Self, ApiError> {
        let cache_manager = CacheManager::new()?;
        let client = Self::build_client()?;
        Ok(Self {
            client,
            config,
            cache_manager,
        })
    }
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
    pub fn get_cached_session(&self) -> Result<Option<CacheData>, ApiError> {
        self.cache_manager.load()
    }
    pub fn cache_file_path(&self) -> std::path::PathBuf {
        self.cache_manager.cache_file_path().to_path_buf()
    }
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
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
