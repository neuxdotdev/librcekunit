//! Client for interacting with the "Input User" management pages.
//!
//! This module provides the [`InputUserClient`] struct, which handles all operations related
//! to the input user section of the application. This includes:
//! - Fetching paginated lists of input users with search, sort, and date filters.
//! - Exporting input user data in various formats (Excel, PDF, CSV, etc.).
//! - Retrieving CSRF tokens for subsequent requests.
//!
//! All methods require an authenticated session; the client uses the cached session
//! (from a previous login) to attach cookies and appropriate headers automatically.

use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;

/// Client for input user operations.
///
/// This client provides methods to interact with the input user listing and export
/// functionality. It relies on a valid session stored in the cache, which must be
/// obtained by first logging in via [`LoginClient`](crate::api::auth::LoginClient).
///
/// # Example
/// ```no_run
/// use cekunit_client::api::dashboard::InputUserClient;
///
/// let client = InputUserClient::new()?;
///
/// // Fetch the first page of input users, sorted by creation date descending
/// let html = client.get_input_user(
///     Some(1),
///     None,
///     Some("created_at"),
///     Some("desc"),
///     None,
///     None
/// )?;
///
/// // Export all input users as Excel
/// let excel_data = client.export_input_user(
///     "excel",
///     "created_at",
///     "desc",
///     None,
///     None,
///     None
/// )?;
/// # Ok::<(), cekunit_client::handler::error::ApiError>(())
/// ```
pub struct InputUserClient {
    /// The underlying reqwest blocking client.
    client: Client,
    /// Environment configuration (base URL, endpoints).
    config: EnvConfig,
    /// Cache manager for loading the session (cookies + CSRF token).
    cache_manager: CacheManager,
}

impl InputUserClient {
    /// Creates a new `InputUserClient` with default configuration loaded from environment variables.
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

    /// Creates a new `InputUserClient` with a given configuration.
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

    /// Creates a new `InputUserClient` with a given configuration and an existing cache manager.
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
    /// - A 120‑second timeout (longer for possible large exports).
    /// - Support for gzip, Brotli, and Deflate compression.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the client builder fails.
    fn build_client() -> Result<Client, ApiError> {
        Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .cookie_store(true)
            .timeout(Duration::from_secs(120))
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

    /// Builds a [`HeaderMap`] containing the User-Agent, Referer, Accept, and the `Cookie` header
    /// derived from the cached session.
    ///
    /// The Referer header is set to the input user URL to mimic a real browser workflow.
    /// Accept is set to `*/*` to accept any response type.
    ///
    /// # Arguments
    /// * `cache` - The cached session data containing cookies.
    ///
    /// # Errors
    /// Returns [`ApiError::CacheError`] if the Referer header value is invalid
    /// (should never happen under normal circumstances).
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

    /// Fetches the input user list HTML with optional pagination, search, sorting, and date filters.
    ///
    /// This method sends a GET request to the input user endpoint and returns the raw HTML
    /// of the listing page.
    ///
    /// # Arguments
    /// * `page` - Optional page number (1‑based). If `None`, the first page is assumed.
    /// * `search` - Optional search term to filter records.
    /// * `sort` - Optional column name to sort by (e.g., `"created_at"`).
    /// * `direction` - Optional sort direction (`"asc"` or `"desc"`).
    /// * `start_date` - Optional start date filter (format depends on server, e.g., `"YYYY-MM-DD"`).
    /// * `end_date` - Optional end date filter.
    ///
    /// # Returns
    /// The raw HTML of the input user list as a `String`.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails (network, timeout).
    /// - The server returns a non‑success status (4xx or 5xx).
    /// - The response body cannot be read.
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
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if status.is_success() {
            let body = response.text().map_err(|e| ApiError::from(e))?;
            Ok(body)
        } else {
            let error_body = response.text().unwrap_or_default();
            Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, error_body
            )))
        }
    }

    /// Exports input user data in the specified format.
    ///
    /// This method sends a GET request to the export endpoint and returns the raw binary data
    /// of the exported file (e.g., Excel, PDF, CSV).
    ///
    /// # Arguments
    /// * `format` - Export format (e.g., `"excel"`, `"pdf"`, `"csv"`). Supported values depend on the server.
    /// * `sort` - Column to sort by (e.g., `"created_at"`).
    /// * `direction` - Sort direction (`"asc"` or `"desc"`).
    /// * `search` - Optional search term to filter records.
    /// * `start_date` - Optional start date filter.
    /// * `end_date` - Optional end date filter.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the exported file data.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails.
    /// - The server returns a non‑success status.
    /// - The response body cannot be read.
    ///
    /// # Example
    /// ```
    /// # use cekunit_client::api::dashboard::InputUserClient;
    /// # let client = InputUserClient::new().unwrap();
    /// let excel_bytes = client.export_input_user(
    ///     "excel",
    ///     "created_at",
    ///     "desc",
    ///     Some("john"),
    ///     Some("2025-01-01"),
    ///     Some("2025-01-31")
    /// )?;
    /// std::fs::write("export.xlsx", excel_bytes)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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
            .map_err(|e| ApiError::from(e))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().unwrap_or_default();
            return Err(ApiError::RequestFailed(format!(
                "HTTP {} - {}",
                status, error_body
            )));
        }

        let mut buf = Vec::new();
        response
            .read_to_end(&mut buf)
            .map_err(|e| ApiError::from(e))?;
        Ok(buf)
    }

    /// Fetches a fresh CSRF token from the input user list page.
    ///
    /// This method retrieves the first page of the input user list using
    /// [`get_input_user`](Self::get_input_user) and extracts the CSRF token from the HTML.
    /// The token can be used for subsequent POST requests if needed.
    ///
    /// # Returns
    /// The CSRF token as a string.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The input user page cannot be fetched.
    /// - No CSRF token is found in the HTML.
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_input_user(Some(1), None, None, None, None, None)?;
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
