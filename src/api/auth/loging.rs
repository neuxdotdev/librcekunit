use crate::api::auth::utils::{
    cache::{CacheData, CacheManager, Cookie},
    cookies::{add_cookies_to_headers, extract_cookies},
    token::extract_csrf_token,
};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
pub struct LoginClient {
    pub client: Client,
    pub config: EnvConfig,
    pub cache_manager: CacheManager,
}
impl LoginClient {
    pub fn new() -> Result<Self, ApiError> {
        let config = EnvConfig::load()?;
        let cache_manager = CacheManager::new()?;
        let client = Self::build_client()?;
        Ok(Self {
            client,
            config,
            cache_manager,
        })
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
    pub fn build_client() -> Result<Client, ApiError> {
        Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0")
            .cookie_store(true)
            .build()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))
    }
    pub fn login(&mut self) -> Result<CacheData, ApiError> {
        log::info!(
            "Starting login process to: {}",
            self.config.full_login_url()
        );
        let csrf_token = self.get_csrf_token()?;
        log::debug!("CSRF token fetched: {}", &csrf_token[..10]);
        let mut login_form: HashMap<&str, &str> = HashMap::new();
        login_form.insert("_token", csrf_token.as_str());
        login_form.insert("email", self.config.user_email.as_str());
        login_form.insert("password", self.config.user_password.as_str());
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0"
                .parse()
                .unwrap(),
        );
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );
        if let Some(cache) = self.cache_manager.load()? {
            let cookie_map: HashMap<String, String> = cache
                .cookies
                .iter()
                .map(|c| (c.name.clone(), c.value.clone()))
                .collect();
            add_cookies_to_headers(&mut headers, &cookie_map)?;
            log::debug!("Loaded {} cached cookies", cookie_map.len());
        }
        log::info!("Sending login request...");
        let response = self
            .client
            .post(self.config.full_login_url())
            .headers(headers)
            .form(&login_form)
            .send()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        let status = response.status();
        log::debug!("Login response status: {}", status);
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            let clean = body.split('<').next().unwrap_or("Unknown error");
            return Err(ApiError::LoginFailed(format!("HTTP {}: {}", status, clean)));
        }
        let cookies = extract_cookies(response.headers());
        log::debug!("Received {} cookies", cookies.len());
        let cache_data = CacheData {
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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        self.cache_manager.save(&cache_data)?;
        log::info!(
            "Login successful. Cache saved at {:?}",
            self.cache_manager.get_cache_file_path()
        );
        Ok(cache_data)
    }
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        log::debug!("Fetching CSRF token from login page");
        let response = self
            .client
            .get(self.config.full_login_url())
            .send()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        let html = response
            .text()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        extract_csrf_token(&html)
    }
    pub fn get_cached_session(&self) -> Result<Option<CacheData>, ApiError> {
        self.cache_manager.load()
    }
    pub fn cache_file_path(&self) -> std::path::PathBuf {
        self.cache_manager.get_cache_file_path()
    }
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }
}
