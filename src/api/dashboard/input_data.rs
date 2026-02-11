use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;
pub struct InputDataClient {
    client: Client,
    config: EnvConfig,
    cache_manager: CacheManager,
}
impl InputDataClient {
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
    pub fn with_config_and_cache(
        config: EnvConfig,
        cache_manager: CacheManager,
    ) -> Result<Self, ApiError> {
        let client = Self::build_client()?;
        Ok(Self {
            client,
            config,
            cache_manager,
        })
    }
    fn build_client() -> Result<Client, ApiError> {
        Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .cookie_store(true)
            .timeout(Duration::from_secs(60))
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .build()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))
    }
    fn ensure_authenticated(&self) -> Result<CacheData, ApiError> {
        match self.cache_manager.load()? {
            Some(cache) if cache.logged_in => Ok(cache),
            _ => Err(ApiError::NotAuthenticated),
        }
    }
    fn build_headers_with_cookies(&self, cache: &CacheData) -> Result<HeaderMap, ApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                .parse()
                .unwrap(),
        );
        let cookie_map: HashMap<String, String> = cache
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect();
        crate::api::auth::utils::cookies::add_cookies_to_headers(&mut headers, &cookie_map)?;
        Ok(headers)
    }
    pub fn get_form(&self) -> Result<String, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;
        let url = self.config.full_input_data_url();
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        let status = response.status();
        if status.is_success() {
            Ok(response
                .text()
                .map_err(|e| ApiError::RequestFailed(Box::new(e)))?)
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(
                format!("HTTP {} - {}", status, body).into(),
            ))
        }
    }
    pub fn insert_nasabah(&self, data: HashMap<&str, &str>) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );
        let url = self.config.full_input_data_url();
        let mut form = HashMap::new();
        form.insert("_token", cache.csrf_token.as_str());
        for (key, value) in data {
            form.insert(key, value);
        }
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        let status = response.status();
        if status.is_success() || status.as_u16() == 302 {
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(
                format!("HTTP {} - {}", status, body).into(),
            ))
        }
    }
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_form()?;
        crate::api::auth::utils::token::extract_csrf_token(&html)
    }
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
}
