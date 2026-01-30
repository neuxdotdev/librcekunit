use crate::api::auth::utils::{
    cache::{CacheData, CacheManager},
    cookies::add_cookies_to_headers,
};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use serde_json::json;
use std::collections::HashMap;
pub struct LogoutClient {
    pub client: Client,
    pub config: EnvConfig,
    pub cache_manager: CacheManager,
}
impl LogoutClient {
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
    pub fn logout(&mut self) -> Result<(), ApiError> {
        log::info!("Starting logout process...");
        let cache_data = match self.cache_manager.load()? {
            Some(data) if data.logged_in => {
                log::debug!("Found active session with {} cookies", data.cookies.len());
                data
            }
            Some(_) => {
                log::warn!("Session exists but not logged in");
                self.cache_manager.clear()?;
                return Ok(());
            }
            None => {
                log::warn!("No active session found");
                return Ok(());
            }
        };
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
        let cookie_map: HashMap<String, String> = cache_data
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect();
        add_cookies_to_headers(&mut headers, &cookie_map)?;
        let logout_data = json!({
            "_token": cache_data.csrf_token,
        });
        log::info!(
            "Sending logout request to: {}",
            self.config.full_logout_url()
        );
        let response = self
            .client
            .post(self.config.full_logout_url())
            .headers(headers)
            .form(&logout_data)
            .send()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        log::debug!("Logout response status: {}", response.status());
        self.cache_manager.clear()?;
        log::info!("Cache cleared successfully");
        if response.status().is_success() {
            log::info!("Logout successful!");
            Ok(())
        } else {
            Err(ApiError::LogoutFailed(format!(
                "HTTP {}",
                response.status()
            )))
        }
    }
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }
    pub fn clear_cache(&mut self) -> Result<(), ApiError> {
        self.cache_manager.clear()
    }
    pub fn load_cache(&self) -> Result<Option<CacheData>, ApiError> {
        self.cache_manager.load()
    }
}
