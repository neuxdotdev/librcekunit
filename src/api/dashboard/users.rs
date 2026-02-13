use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;
pub struct UsersClient {
    client: Client,
    config: EnvConfig,
    cache_manager: CacheManager,
}
impl UsersClient {
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
            .timeout(Duration::from_secs(30))
            .connection_verbose(true)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| {
                log::error!("❌ Failed to build HTTP client: {}", e);
                ApiError::RequestFailed(e.to_string())
            })
    }
    fn ensure_authenticated(&self) -> Result<CacheData, ApiError> {
        match self.cache_manager.load()? {
            Some(cache) if cache.logged_in => {
                log::debug!("🔐 Valid session loaded ({} cookies)", cache.cookies.len());
                Ok(cache)
            }
            Some(_) => {
                log::warn!("⚠️ Session exists but not logged in – clearing cache");
                self.cache_manager.clear()?;
                Err(ApiError::NotAuthenticated)
            }
            None => {
                log::warn!("⚠️ No active session found");
                Err(ApiError::NotAuthenticated)
            }
        }
    }
    fn build_headers_with_cookies(&self, cache: &CacheData) -> Result<HeaderMap, ApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                .parse()
                .map_err(|e| ApiError::CacheError(format!("Invalid User-Agent: {}", e)))?,
        );
        let cookie_map: HashMap<String, String> = cache
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect();
        if !cookie_map.is_empty() {
            crate::api::auth::utils::cookies::add_cookies_to_headers(&mut headers, &cookie_map)?;
            log::debug!("🍪 Attached {} cookies", cookie_map.len());
        }
        Ok(headers)
    }
    pub fn get_users_list(
        &self,
        page: Option<u32>,
        sort: Option<&str>,
        direction: Option<&str>,
    ) -> Result<String, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;
        let mut url = self.config.full_users_url();
        let mut params = Vec::new();
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(s) = sort {
            params.push(format!("sort={}", s));
        }
        if let Some(d) = direction {
            params.push(format!("direction={}", d));
        }
        if !params.is_empty() {
            url.push_str("?");
            url.push_str(&params.join("&"));
        }
        log::debug!("📤 Requesting users list: {}", url);
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| ApiError::from_reqwest_error(e, "GET users"))?;
        let status = response.status();
        if status.is_success() {
            let html = response
                .text()
                .map_err(|e| ApiError::RequestFailed(e.to_string()))?;
            log::debug!("✅ Users list fetched, {} bytes", html.len());
            Ok(html)
        } else {
            let body = response.text().unwrap_or_default();
            let err = ApiError::from_status(status, Some(&body));
            log::error!("❌ Failed to fetch users list: {}", err);
            Err(err)
        }
    }
    pub fn update_user(&self, id: &str, data: HashMap<&str, &str>) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded"
                .parse()
                .map_err(|e| ApiError::CacheError(format!("Invalid Content-Type: {}", e)))?,
        );
        let url = self.config.full_users_item_url(id);
        let mut form: HashMap<&str, &str> = HashMap::new();
        form.insert("_token", cache.csrf_token.as_str());
        form.insert("_method", "PUT");
        for (key, value) in data {
            form.insert(key, value);
        }
        log::info!("📤 Updating user {} at {}", id, url);
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .map_err(|e| ApiError::from_reqwest_error(e, "PUT user"))?;
        let status = response.status();
        if status.is_success() || status.as_u16() == 302 {
            log::info!("✅ User {} updated successfully", id);
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            let err = ApiError::from_status(status, Some(&body));
            log::error!("❌ Failed to update user {}: {}", id, err);
            Err(err)
        }
    }
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_users_list(Some(1), None, None)?;
        crate::api::auth::utils::token::extract_csrf_token(&html)
    }
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
}
