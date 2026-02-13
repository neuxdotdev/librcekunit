//! Client for interacting with PIC (Person In Charge) management endpoints.
//!
//! This module provides the [`PicClient`] struct, which handles all operations related
//! to PIC (Person In Charge) entities in the CekUnit application. It supports:
//! - Fetching a paginated list of PICs with sorting options.
//! - Creating a new PIC record.
//! - Updating an existing PIC record.
//! - Deleting a PIC record.
//! - Retrieving CSRF tokens for form submissions.
//!
//! All methods require an authenticated session; the client uses the cached session
//! (from a previous login) to attach cookies and CSRF tokens automatically.

use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;

/// Client for PIC (Person In Charge) operations.
///
/// This client provides methods to list, create, update, and delete PIC records.
/// It relies on a valid session stored in the cache, which must be obtained
/// by first logging in via [`LoginClient`](crate::api::auth::LoginClient).
///
/// # Example
/// ```no_run
/// use cekunit_client::api::dashboard::PicClient;
/// use std::collections::HashMap;
///
/// let client = PicClient::new()?;
///
/// // Fetch the first page of PIC list, sorted by name ascending
/// let html = client.get_pic_list(Some(1), Some("name"), Some("asc"))?;
///
/// // Prepare data for a new PIC
/// let mut data = HashMap::new();
/// data.insert("name", "John Doe");
/// data.insert("email", "john@example.com");
/// data.insert("phone", "123456789");
///
/// // Insert the new PIC
/// client.insert_pic(data)?;
///
/// // Update an existing PIC (ID = 5)
/// let mut update_data = HashMap::new();
/// update_data.insert("name", "Jane Doe");
/// client.update_pic("5", update_data)?;
///
/// // Delete a PIC
/// client.delete_pic("5")?;
/// # Ok::<(), cekunit_client::handler::error::ApiError>(())
/// ```
pub struct PicClient {
    /// The underlying reqwest blocking client.
    client: Client,
    /// Environment configuration (base URL, endpoints).
    config: EnvConfig,
    /// Cache manager for loading the session (cookies + CSRF token).
    cache_manager: CacheManager,
}

impl PicClient {
    /// Creates a new `PicClient` with default configuration loaded from environment variables.
    ///
    /// This is a convenience constructor that loads the configuration and creates a cache manager.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - Environment variables are missing or invalid.
    /// - The cache directory cannot be created.
    /// - The HTTP client cannot be built.
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

    /// Creates a new `PicClient` with a given configuration.
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

    /// Creates a new `PicClient` with a given configuration and an existing cache manager.
    ///
    /// This is useful when sharing the same cache (and thus the same session) across multiple clients.
    ///
    /// # Arguments
    /// * `config` - The environment configuration.
    /// * `cache_manager` - An existing cache manager (typically from the main client).
    ///
    /// # Errors
    /// Returns [`ApiError`] if the HTTP client cannot be built.
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

    /// Builds and configures the HTTP client.
    ///
    /// The client is configured with:
    /// - A Chrome‑like User-Agent.
    /// - Automatic cookie storage.
    /// - A 60‑second timeout for all requests.
    /// - Support for gzip, Brotli, and Deflate compression.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the client builder fails.
    fn build_client() -> Result<Client, ApiError> {
        Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .cookie_store(true)
            .timeout(Duration::from_secs(60))
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .build()
            .map_err(|e| ApiError::from(e))
    }

    /// Ensures that a valid authenticated session exists in the cache.
    ///
    /// Loads the cache and checks the `logged_in` flag. If the session is valid,
    /// returns the [`CacheData`]. Otherwise returns [`ApiError::NotAuthenticated`].
    ///
    /// # Errors
    /// - [`ApiError::NotAuthenticated`] if no cache exists or `logged_in` is false.
    /// - [`ApiError::CacheError`] if loading the cache fails.
    fn ensure_authenticated(&self) -> Result<CacheData, ApiError> {
        match self.cache_manager.load()? {
            Some(cache) if cache.logged_in => Ok(cache),
            _ => Err(ApiError::NotAuthenticated),
        }
    }

    /// Builds a [`HeaderMap`] containing the User-Agent and the `Cookie` header
    /// derived from the cached session.
    ///
    /// # Arguments
    /// * `cache` - The cached session data containing cookies.
    ///
    /// # Errors
    /// Returns [`ApiError::CacheError`] if the cookie header cannot be constructed
    /// (should never happen under normal circumstances).
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

    /// Fetches the PIC list HTML with optional pagination and sorting.
    ///
    /// This method sends a GET request to the PIC listing endpoint and returns the raw HTML
    /// of the page.
    ///
    /// # Arguments
    /// * `page` - Optional page number (1‑based). If `None`, the first page is assumed.
    /// * `sort` - Optional column name to sort by (e.g., `"name"`, `"created_at"`).
    /// * `direction` - Optional sort direction (`"asc"` or `"desc"`).
    ///
    /// # Returns
    /// The raw HTML of the PIC list as a `String`.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails (network, timeout).
    /// - The server returns a non‑success status (4xx or 5xx).
    /// - The response body cannot be read.
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

    /// Creates a new PIC record.
    ///
    /// This method sends a POST request to the input PIC endpoint with the provided form data.
    /// The CSRF token from the cached session is automatically included as `_token`.
    /// The caller must provide all required fields for the new PIC.
    ///
    /// # Arguments
    /// * `data` - A map of field names to values. The map **must not** include the `_token` field,
    ///            as it is added automatically.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status (2xx or 302 is considered success).
    ///
    /// # Example
    /// ```
    /// # use std::collections::HashMap;
    /// # use cekunit_client::api::dashboard::PicClient;
    /// # let client = PicClient::new().unwrap();
    /// let mut new_pic = HashMap::new();
    /// new_pic.insert("name", "Alice Smith");
    /// new_pic.insert("email", "alice@example.com");
    /// new_pic.insert("phone", "555-1234");
    /// client.insert_pic(new_pic)?;
    /// # Ok::<(), cekunit_client::handler::error::ApiError>(())
    /// ```
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

    /// Updates an existing PIC record.
    ///
    /// This method sends a POST request with `_method=PUT` to the PIC item endpoint.
    /// The CSRF token is automatically included, and the caller provides the fields to update.
    ///
    /// # Arguments
    /// * `id` - The identifier of the PIC to update.
    /// * `data` - A map of field names to new values. The map **must not** include `_token` or `_method`.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status (2xx or 302 is considered success).
    ///
    /// # Example
    /// ```
    /// # use std::collections::HashMap;
    /// # use cekunit_client::api::dashboard::PicClient;
    /// # let client = PicClient::new().unwrap();
    /// let mut updates = HashMap::new();
    /// updates.insert("name", "Robert Johnson");
    /// client.update_pic("10", updates)?;
    /// # Ok::<(), cekunit_client::handler::error::ApiError>(())
    /// ```
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

    /// Deletes a PIC record.
    ///
    /// This method sends a POST request with `_method=DELETE` to the PIC item endpoint.
    ///
    /// # Arguments
    /// * `id` - The identifier of the PIC to delete.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status (2xx or 302 is considered success).
    ///
    /// # Warning
    /// This operation is irreversible. Use with caution.
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

    /// Fetches a fresh CSRF token from the PIC list page.
    ///
    /// This method retrieves the first page of the PIC list using
    /// [`get_pic_list`](Self::get_pic_list) and extracts the CSRF token from the HTML.
    /// The token can be used for subsequent POST requests if needed.
    ///
    /// # Returns
    /// The CSRF token as a string.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The PIC list page cannot be fetched.
    /// - No CSRF token is found in the HTML.
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_pic_list(Some(1), None, None)?;
        crate::api::auth::utils::token::extract_csrf_token(&html)
    }

    /// Returns a reference to the environment configuration.
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }

    /// Returns a reference to the cache manager.
    pub fn cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
}
