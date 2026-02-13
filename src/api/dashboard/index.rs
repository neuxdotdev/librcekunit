use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;

pub struct DashboardClient {
    client: Client,
    config: EnvConfig,
    cache_manager: CacheManager,
}

impl DashboardClient {
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
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0")
            .cookie_store(true)
            .build()
            .map_err(|e| ApiError::from(e))
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
            "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0"
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

    pub fn get_dashboard(
        &self,
        page: Option<u32>,
        search: Option<&str>,
        sort: Option<&str>,
        direction: Option<&str>,
    ) -> Result<String, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;

        let mut url = self.config.full_dashboard_url();
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

        if !params.is_empty() {
            url.push_str("?");
            url.push_str(&params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() {
            Ok(response.text().map_err(|e| ApiError::from(e))?)
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, body
            )))
        }
    }

    pub fn export_cekunit(
        &self,
        format: &str,
        sort: &str,
        direction: &str,
    ) -> Result<Vec<u8>, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;

        let url = format!(
            "{}?format={}&sort={}&direction={}",
            self.config.full_cekunit_export_url(),
            format,
            sort,
            direction
        );

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() {
            Ok(response.bytes().map_err(|e| ApiError::from(e))?.to_vec())
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, body
            )))
        }
    }

    pub fn get_unique_values(&self, column: &str) -> Result<Vec<String>, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;

        let url = format!(
            "{}?column={}",
            self.config.full_cekunit_unique_url(),
            column
        );

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() {
            let values: Vec<String> = response.json().map_err(|e| ApiError::from(e))?;
            Ok(values)
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, body
            )))
        }
    }

    pub fn delete_by_category(&self, column: &str, value: &str) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let url = self.config.full_cekunit_delete_category_url();
        let mut form = HashMap::new();
        form.insert("_token", cache.csrf_token.as_str());
        form.insert("column", column);
        form.insert("value", value);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, body
            )))
        }
    }

    pub fn delete_all(&self) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let url = self.config.full_delete_all_url();
        let mut form = HashMap::new();
        form.insert("_token", cache.csrf_token.as_str());
        form.insert("_method", "DELETE");

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 302 {
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, body
            )))
        }
    }

    pub fn delete_cekunit(&self, no: &str) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let url = self.config.full_cekunit_item_url(no);
        let mut form = HashMap::new();
        form.insert("_token", cache.csrf_token.as_str());
        form.insert("_method", "DELETE");

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 302 {
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, body
            )))
        }
    }

    pub fn update_cekunit(&self, no: &str, data: HashMap<&str, &str>) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let url = self.config.full_cekunit_item_url(no);
        let mut form: HashMap<&str, &str> = HashMap::new();
        form.insert("_token", cache.csrf_token.as_str());
        form.insert("_method", "PUT");
        for (key, value) in data {
            form.insert(key, value);
        }

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 302 {
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, body
            )))
        }
    }

    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_dashboard(Some(1), None, None, None)?;
        crate::api::auth::utils::token::extract_csrf_token(&html)
    }

    pub fn config(&self) -> &EnvConfig {
        &self.config
    }

    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
}
