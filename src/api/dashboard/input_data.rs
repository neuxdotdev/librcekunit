//! Client for interacting with the "Input Data" (Nasabah) forms.
//!
//! This module provides the [`InputDataClient`] struct, which handles operations related
//! to the input data forms, specifically for inserting new nasabah (customer) records.
//! It allows fetching the form HTML and submitting the form data.
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

/// Client for input data (nasabah) operations.
///
/// This client provides methods to fetch the input form HTML and to submit new nasabah
/// records. It relies on a valid session stored in the cache, which must be obtained
/// by first logging in via [`LoginClient`](crate::api::auth::LoginClient).
///
/// # Example
/// ```no_run
/// use cekunit_client::api::dashboard::InputDataClient;
/// use std::collections::HashMap;
///
/// let client = InputDataClient::new()?;
///
/// // Fetch the form HTML (for inspection or to extract a fresh CSRF token)
/// let form_html = client.get_form()?;
///
/// // Prepare data for a new nasabah
/// let mut data = HashMap::new();
/// data.insert("nama", "John Doe");
/// data.insert("alamat", "123 Main St");
/// data.insert("no_ktp", "1234567890");
///
/// // Submit the form
/// client.insert_nasabah(data)?;
/// # Ok::<(), cekunit_client::handler::error::ApiError>(())
/// ```
pub struct InputDataClient {
    /// The underlying reqwest blocking client.
    client: Client,
    /// Environment configuration (base URL, endpoints).
    config: EnvConfig,
    /// Cache manager for loading the session (cookies + CSRF token).
    cache_manager: CacheManager,
}

impl InputDataClient {
    /// Creates a new `InputDataClient` with default configuration loaded from environment variables.
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

    /// Creates a new `InputDataClient` with a given configuration.
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

    /// Creates a new `InputDataClient` with a given configuration and an existing cache manager.
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

    /// Fetches the input data form HTML.
    ///
    /// This method sends a GET request to the input data endpoint and returns the raw HTML
    /// of the form page. The HTML can be used to extract a fresh CSRF token or to inspect
    /// the form structure.
    ///
    /// # Returns
    /// The HTML content as a `String`.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails (network, timeout).
    /// - The server returns a non‑success status (4xx or 5xx).
    /// - The response body cannot be read.
    pub fn get_form(&self) -> Result<String, ApiError> {
        let cache = self.ensure_authenticated()?;
        let headers = self.build_headers_with_cookies(&cache)?;
        let url = self.config.full_input_data_url();

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

    /// Submits a new nasabah (customer) record via the input data form.
    ///
    /// This method sends a POST request to the input data endpoint with the provided form data.
    /// The CSRF token from the cached session is automatically included as `_token`.
    /// The caller must provide all required fields for the nasabah record.
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
    /// # use cekunit_client::api::dashboard::InputDataClient;
    /// # let client = InputDataClient::new().unwrap();
    /// let mut nasabah_data = HashMap::new();
    /// nasabah_data.insert("nama", "Jane Doe");
    /// nasabah_data.insert("alamat", "456 Oak Ave");
    /// nasabah_data.insert("no_ktp", "9876543210");
    /// nasabah_data.insert("tanggal_lahir", "1990-01-01");
    ///
    /// client.insert_nasabah(nasabah_data)?;
    /// # Ok::<(), cekunit_client::handler::error::ApiError>(())
    /// ```
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

    /// Fetches a fresh CSRF token from the input data form page.
    ///
    /// This method retrieves the form HTML using [`get_form`](Self::get_form) and extracts
    /// the CSRF token from it. The token can be used for subsequent POST requests if needed,
    /// though the client automatically uses the cached token for submissions.
    ///
    /// # Returns
    /// The CSRF token as a string.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The form page cannot be fetched.
    /// - No CSRF token is found in the HTML.
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_form()?;
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
