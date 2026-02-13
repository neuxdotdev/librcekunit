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
const USER_AGENT_STR: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(100);
pub struct LogoutClient {
    pub client: Client,
    pub config: EnvConfig,
    pub cache_manager: CacheManager,
}
impl LogoutClient {
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
                log::error!(" Failed to build HTTP client: {}", e);
                ApiError::from(e)
            })
    }
    pub fn logout(&mut self) -> Result<(), ApiError> {
        log::info!(" Starting logout process (using cached token)");
        let cache_data = self.load_valid_session()?;
        let headers = self.build_headers(&cache_data)?;
        let mut form = HashMap::new();
        form.insert("_token", cache_data.csrf_token.as_str());
        self.execute_logout_request(headers, form)
    }
    pub fn logout_with_token(&mut self, csrf_token: &str) -> Result<(), ApiError> {
        log::info!(" Starting logout process (using provided token)");
        let cache_data = self.load_valid_session()?;
        let headers = self.build_headers(&cache_data)?;
        let mut form = HashMap::new();
        form.insert("_token", csrf_token);
        self.execute_logout_request(headers, form)
    }
    pub fn clear_cache(&mut self) -> Result<(), ApiError> {
        log::info!(" Clearing cache manually");
        self.cache_manager.clear()
    }
    pub fn load_cache(&self) -> Result<Option<CacheData>, ApiError> {
        self.cache_manager.load()
    }
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
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
