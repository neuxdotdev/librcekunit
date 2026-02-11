use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;
pub struct InputUserClient {
    client: Client,
    config: EnvConfig,
    cache_manager: CacheManager,
}
impl InputUserClient {
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
            .timeout(Duration::from_secs(120))
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
            HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        );
        headers.insert(
            REFERER,
            HeaderValue::from_str(&self.config.full_input_user_url())
                .map_err(|e| ApiError::CacheError(format!("Invalid referer: {}", e)))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        let cookie_map: HashMap<String, String> = cache
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect();
        crate::api::auth::utils::cookies::add_cookies_to_headers(&mut headers, &cookie_map)?;
        Ok(headers)
    }
    pub fn get_input_user(
        &self,
        page: Option<u32>,
        search: Option<&str>,
        sort: Option<&str>,
        direction: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<String, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;
        let mut url = self.config.full_input_user_url();
        let mut params = Vec::new();
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(s) = search {
            params.push(format!("search={}", s));
        }
        if let Some(s) = sort {
            params.push(format!("sort={}", s));
        }
        if let Some(d) = direction {
            params.push(format!("direction={}", d));
        }
        if let Some(sd) = start_date {
            params.push(format!("start_date={}", sd));
        }
        if let Some(ed) = end_date {
            params.push(format!("end_date={}", ed));
        }
        if !params.is_empty() {
            url.push_str("?");
            url.push_str(&params.join("&"));
        }
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        let status = response.status();
        if status.is_success() {
            let body = response
                .text()
                .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
            Ok(body)
        } else {
            let error_body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(
                format!("HTTP {} - {}", status, error_body).into(),
            ))
        }
    }
    pub fn export_input_user(
        &self,
        format: &str,
        sort: &str,
        direction: &str,
        search: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<u8>, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;
        let mut url = self.config.full_input_user_export_url();
        let mut params = vec![
            format!("format={}", format),
            format!("sort={}", sort),
            format!("direction={}", direction),
        ];
        if let Some(s) = search {
            params.push(format!("search={}", s));
        }
        if let Some(sd) = start_date {
            params.push(format!("start_date={}", sd));
        }
        if let Some(ed) = end_date {
            params.push(format!("end_date={}", ed));
        }
        url.push('?');
        url.push_str(&params.join("&"));
        let mut response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().unwrap_or_default();
            return Err(ApiError::RequestFailed(
                format!("HTTP {} - {}", status, error_body).into(),
            ));
        }
        let mut buf = Vec::new();
        response
            .read_to_end(&mut buf)
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))?;
        Ok(buf)
    }
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_input_user(Some(1), None, None, None, None, None)?;
        crate::api::auth::utils::token::extract_csrf_token(&html)
    }
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
}
