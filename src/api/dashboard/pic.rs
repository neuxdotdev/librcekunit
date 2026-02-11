use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;

use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;

pub struct PicClient {
    client: Client,
    config: EnvConfig,
    cache_manager: CacheManager,
}

impl PicClient {
    // ==================== CONSTRUCTORS ====================
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

    // ==================== PRIVATE HELPERS ====================
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

    // ==================== PUBLIC API ====================

    /// GET /dashboard/pic – mengambil halaman daftar PIC
    ///
    /// # Parameters
    /// * `page` - Halaman yang akan ditampilkan
    /// * `sort` - Kolom sorting (opsional, dari HTML tidak ada sorting, tapi endpoint mendukung)
    /// * `direction` - Arah sorting (asc/desc)
    pub fn get_pic_list(
        &self,
        page: Option<u32>,
        sort: Option<&str>,
        direction: Option<&str>,
    ) -> Result<String, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;

        let mut url = self.config.full_pic_url();
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

    /// POST /dashboard/input-PIC – menambahkan data PIC baru
    ///
    /// # Arguments
    /// * `data` - HashMap dengan key = nama field, value = nilai
    ///   Field yang tersedia: id_coll, nama_collector, no_wa, status
    pub fn insert_pic(&self, data: HashMap<&str, &str>) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let url = self.config.full_input_pic_url();

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

    /// POST /pic/{id} (dengan _method=PUT) – memperbarui data PIC
    ///
    /// # Arguments
    /// * `id` - Nomor PIC (primary key)
    /// * `data` - HashMap dengan field yang ingin diupdate
    pub fn update_pic(&self, id: &str, data: HashMap<&str, &str>) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let url = self.config.full_pic_item_url(id);

        let mut form = HashMap::new();
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

    /// POST /pic/{id} (dengan _method=DELETE) – menghapus data PIC
    pub fn delete_pic(&self, id: &str) -> Result<(), ApiError> {
        let cache = self.ensure_authenticated()?;
        let mut headers = self.build_headers_with_cookies(&cache)?;
        headers.insert(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let url = self.config.full_pic_item_url(id);

        let mut form = HashMap::new();
        form.insert("_token", cache.csrf_token.as_str());
        form.insert("_method", "DELETE");

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

    /// Mendapatkan CSRF token fresh dari halaman PIC (opsional)
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_pic_list(Some(1), None, None)?;
        crate::api::auth::utils::token::extract_csrf_token(&html)
    }

    // ==================== ACCESSORS ====================
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }

    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
}
