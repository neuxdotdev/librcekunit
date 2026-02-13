//! Client for interacting with user management endpoints.
//!
//! This module provides the [`UsersClient`] struct, which handles operations related
//! to application users (not to be confused with input users). It supports:
//! - Fetching a paginated list of users with sorting options.
//! - Updating an existing user's details.
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

/// Client for user management operations.
///
/// This client provides methods to list and update application users.
/// It relies on a valid session stored in the cache, which must be obtained
/// by first logging in via [`LoginClient`](crate::api::auth::LoginClient).
///
/// # Example
/// ```no_run
/// use cekunit_client::api::dashboard::UsersClient;
/// use std::collections::HashMap;
///
/// let client = UsersClient::new()?;
///
/// // Fetch the first page of users, sorted by name ascending
/// let html = client.get_users_list(Some(1), Some("name"), Some("asc"))?;
///
/// // Update an existing user (ID = 42)
/// let mut updates = HashMap::new();
/// updates.insert("name", "New Name");
/// updates.insert("email", "new@example.com");
/// client.update_user("42", updates)?;
/// # Ok::<(), cekunit_client::handler::error::ApiError>(())
/// ```
pub struct UsersClient {
    /// The underlying reqwest blocking client.
    client: Client,
    /// Environment configuration (base URL, endpoints).
    config: EnvConfig,
    /// Cache manager for loading the session (cookies + CSRF token).
    cache_manager: CacheManager,
}

impl UsersClient {
    /// Creates a new `UsersClient` with default configuration loaded from environment variables.
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
        Self::with_config(config)
    }

    /// Creates a new `UsersClient` with a given configuration.
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

    /// Creates a new `UsersClient` with a given configuration and an existing cache manager.
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
    /// - A 30‑second timeout.
    /// - Connection verbose logging (for debugging).
    /// - TCP keepalive (60 seconds).
    /// - Connection pool idle timeout (90 seconds).
    /// - Maximum 10 idle connections per host.
    ///
    /// # Errors
    /// Returns [`ApiError::RequestFailed`] if the client builder fails.
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
                log::error!(" Failed to build HTTP client: {}", e);
                ApiError::RequestFailed(e.to_string())
            })
    }

    /// Ensures that a valid authenticated session exists in the cache.
    ///
    /// Loads the cache and checks the `logged_in` flag. If the session is valid,
    /// returns the [`CacheData`]. If the cache exists but `logged_in` is false,
    /// the cache is cleared and [`ApiError::NotAuthenticated`] is returned.
    /// If no cache exists, returns [`ApiError::NotAuthenticated`].
    ///
    /// # Errors
    /// - [`ApiError::NotAuthenticated`] if no valid logged‑in session is found.
    /// - [`ApiError::CacheError`] if loading or clearing the cache fails.
    fn ensure_authenticated(&self) -> Result<CacheData, ApiError> {
        match self.cache_manager.load()? {
            Some(cache) if cache.logged_in => {
                log::debug!(" Valid session loaded ({} cookies)", cache.cookies.len());
                Ok(cache)
            }
            Some(_) => {
                log::warn!("️ Session exists but not logged in – clearing cache");
                self.cache_manager.clear()?;
                Err(ApiError::NotAuthenticated)
            }
            None => {
                log::warn!("️ No active session found");
                Err(ApiError::NotAuthenticated)
            }
        }
    }

    /// Builds a [`HeaderMap`] containing the User-Agent and the `Cookie` header
    /// derived from the cached session.
    ///
    /// # Arguments
    /// * `cache` - The cached session data containing cookies.
    ///
    /// # Errors
    /// Returns [`ApiError::CacheError`] if the User-Agent header value is invalid
    /// (should never happen under normal circumstances).
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
            log::debug!(" Attached {} cookies", cookie_map.len());
        }

        Ok(headers)
    }

    /// Fetches the users list HTML with optional pagination and sorting.
    ///
    /// This method sends a GET request to the users listing endpoint and returns the raw HTML
    /// of the page.
    ///
    /// # Arguments
    /// * `page` - Optional page number (1‑based). If `None`, the first page is assumed.
    /// * `sort` - Optional column name to sort by (e.g., `"name"`, `"email"`).
    /// * `direction` - Optional sort direction (`"asc"` or `"desc"`).
    ///
    /// # Returns
    /// The raw HTML of the users list as a `String`.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The HTTP request fails (network, timeout).
    /// - The server returns a non‑success status (4xx or 5xx).
    /// - The response body cannot be read.
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

        log::debug!(" Requesting users list: {}", url);

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
            log::debug!(" Users list fetched, {} bytes", html.len());
            Ok(html)
        } else {
            let body = response.text().unwrap_or_default();
            let err = ApiError::from_status(status, Some(&body));
            log::error!(" Failed to fetch users list: {}", err);
            Err(err)
        }
    }

    /// Updates an existing user's details.
    ///
    /// This method sends a POST request with `_method=PUT` to the user item endpoint.
    /// The CSRF token is automatically included, and the caller provides the fields to update.
    ///
    /// # Arguments
    /// * `id` - The identifier of the user to update.
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
    /// # use cekunit_client::api::dashboard::UsersClient;
    /// # let client = UsersClient::new().unwrap();
    /// let mut updates = HashMap::new();
    /// updates.insert("name", "Jane Doe");
    /// updates.insert("email", "jane@example.com");
    /// client.update_user("5", updates)?;
    /// # Ok::<(), cekunit_client::handler::error::ApiError>(())
    /// ```
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

        log::info!(" Updating user {} at {}", id, url);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .map_err(|e| ApiError::from_reqwest_error(e, "PUT user"))?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 302 {
            log::info!(" User {} updated successfully", id);
            Ok(())
        } else {
            let body = response.text().unwrap_or_default();
            let err = ApiError::from_status(status, Some(&body));
            log::error!(" Failed to update user {}: {}", id, err);
            Err(err)
        }
    }

    /// Fetches a fresh CSRF token from the users list page.
    ///
    /// This method retrieves the first page of the users list using
    /// [`get_users_list`](Self::get_users_list) and extracts the CSRF token from the HTML.
    /// The token can be used for subsequent POST requests if needed.
    ///
    /// # Returns
    /// The CSRF token as a string.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - No valid session exists.
    /// - The users list page cannot be fetched.
    /// - No CSRF token is found in the HTML.
    pub fn get_csrf_token(&self) -> Result<String, ApiError> {
        let html = self.get_users_list(Some(1), None, None)?;
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
