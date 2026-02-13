//! Client for interacting with the main dashboard and CekUnit management endpoints.
//!
//! This module provides the [`DashboardClient`] struct, which handles all operations related
//! to the dashboard and CekUnit data, including:
//! - Fetching the dashboard list with pagination, search, sorting.
//! - Exporting CekUnit data in various formats.
//! - Retrieving unique values for filtering.
//! - Deleting records by category or individually.
//! - Updating existing CekUnit records.
//!
//! All methods require an authenticated session; the client uses the cached session
//! (from a previous login) to attach cookies and CSRF tokens automatically.

use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use std::collections::HashMap;

/// Client for dashboard and CekUnit operations.
///
/// This client provides methods to interact with the main dashboard (CekUnit list),
/// as well as to export, delete, and update CekUnit records. It relies on a valid
/// session stored in the cache, which must be obtained by first logging in via
/// [`LoginClient`](crate::api::auth::LoginClient).
///
/// # Example
/// ```no_run
/// use cekunit_client::api::dashboard::DashboardClient;
///
/// let client = DashboardClient::new()?;
/// let html = client.get_dashboard(Some(1), None, Some("created_at"), Some("desc"))?;
/// println!("Dashboard page 1: {}", html);
/// # Ok::<(), cekunit_client::handler::error::ApiError>(())
/// ```
pub struct DashboardClient {
    /// The underlying reqwest blocking client.
    client: Client,
    /// Environment configuration (base URL, endpoints).
    config: EnvConfig,
    /// Cache manager for loading the session (cookies + CSRF token).
    cache_manager: CacheManager,
}

impl DashboardClient {
    /// Creates a new `DashboardClient` with default configuration loaded from environment variables.
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

    /// Creates a new `DashboardClient` with a given configuration.
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

    /// Creates a new `DashboardClient` with a given configuration and an existing cache manager.
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
    /// - A Firefox‑like User-Agent.
    /// - Automatic cookie storage (enabled).
    /// - No explicit timeout (will be added later if needed).
    ///
    /// # Errors
    /// Returns [`ApiError`] if the client builder fails.
    fn build_client() -> Result<Client, ApiError> {
        Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0")
            .cookie_store(true)
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

    /// Fetches the dashboard (CekUnit list) HTML.
    ///
    /// Allows pagination, searching, sorting, and ordering direction.
    ///
    /// # Arguments
    /// * `page` - Optional page number (1‑based). If `None`, the first page is assumed.
    /// * `search` - Optional search term to filter records.
    /// * `sort` - Optional column name to sort by (e.g., `"created_at"`).
    /// * `direction` - Optional sort direction (`"asc"` or `"desc"`).
    ///
    /// # Returns
    /// The raw HTML of the dashboard page as a `String`.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails (network, timeout).
    /// - The server returns a non‑success status (4xx or 5xx).
    /// - The response body cannot be read.
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

    /// Exports CekUnit data in the specified format.
    ///
    /// # Arguments
    /// * `format` - Export format (e.g., `"excel"`, `"pdf"`, `"csv"`). Supported values depend on the server.
    /// * `sort` - Column to sort by (e.g., `"created_at"`).
    /// * `direction` - Sort direction (`"asc"` or `"desc"`).
    ///
    /// # Returns
    /// A `Vec<u8>` containing the exported file data (e.g., Excel binary, PDF bytes, CSV text).
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status.
    /// - The response body cannot be read.
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

    /// Fetches unique values for a given column from the CekUnit data.
    ///
    /// This is typically used to populate filter dropdowns.
    ///
    /// # Arguments
    /// * `column` - The column name for which to retrieve unique values.
    ///
    /// # Returns
    /// A vector of unique string values.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status.
    /// - The response is not valid JSON.
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

    /// Deletes all CekUnit records that match a given category (column = value).
    ///
    /// # Arguments
    /// * `column` - The column name to match (e.g., `"status"`).
    /// * `value` - The value to match (e.g., `"completed"`).
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status.
    ///
    /// # Note
    /// This operation is irreversible. Use with caution.
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

    /// Deletes **all** CekUnit records.
    ///
    /// This sends a POST request with `_method=DELETE` to the delete‑all endpoint.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status (2xx or 302 is considered success).
    ///
    /// # Warning
    /// This operation is extremely destructive and irreversible.
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

    /// Deletes a single CekUnit record identified by its primary key `no`.
    ///
    /// # Arguments
    /// * `no` - The identifier of the record to delete.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status (2xx or 302 is considered success).
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

    /// Updates an existing CekUnit record.
    ///
    /// # Arguments
    /// * `no` - The identifier of the record to update.
    /// * `data` - A map of field names to new values. The map must include the CSRF token
    ///   automatically; the caller should **not** include `_token` or `_method`.
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
    /// # use cekunit_client::api::dashboard::DashboardClient;
    /// # let client = DashboardClient::new().unwrap();
    /// let mut updates = HashMap::new();
    /// updates.insert("status", "approved");
    /// updates.insert("notes", "Updated via API");
    /// client.update_cekunit("123", updates)?;
    /// # Ok::<(), cekunit_client::handler::error::ApiError>(())
    /// ```
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

    /// Fetches a fresh CSRF token from the dashboard page.
    ///
    /// This method retrieves the first page of the dashboard and extracts the CSRF token
    /// from the HTML. It is useful when a token is needed for subsequent POST requests.
    ///
    /// # Returns
    /// The CSRF token as a string.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The dashboard page cannot be fetched.
    /// - No CSRF token is found in the HTML.
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_dashboard(Some(1), None, None, None)?;
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
