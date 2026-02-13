//! Main client for the CekUnit API.
//!
//! This module provides the primary entry point [`CekUnitClient`] for interacting with the
//! CekUnit application. It manages authentication, session caching, and provides access to
//! various sub-clients for different parts of the API (dashboard, input data, PIC, users, etc.).
//!
//! The client is built around a shared context ([`ClientContext`]) that holds the configuration
//! and the session cache. Sub-clients are created on demand using the [`FromContext`] trait,
//! ensuring they all use the same configuration and session data.

use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::api::auth::{LoginClient, LogoutClient};
use crate::api::dashboard::{
    DashboardClient, InputDataClient, InputUserClient, PicClient, UsersClient,
};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use std::path::PathBuf;
use std::sync::Arc;

/// Alias for the environment configuration type.
pub type ConfigType = EnvConfig;

/// Alias for the cache manager type.
pub type CacheManagerType = CacheManager;

/// Shared context for all clients.
///
/// This struct holds the global configuration and the session cache.
/// It is typically wrapped in an [`Arc`] to allow multiple sub-clients to share it safely.
#[derive(Clone)]
pub struct ClientContext {
    /// The environment configuration (base URL, endpoints, credentials).
    pub config: ConfigType,
    /// The cache manager for session persistence.
    pub cache: CacheManagerType,
}

/// Trait for creating a client from a shared context.
///
/// This trait is implemented by all sub-clients that need access to the global
/// configuration and session cache. It allows the main client to create them
/// on demand without coupling to concrete constructors.
pub trait FromContext: Sized {
    /// Creates an instance of `Self` from the given shared context.
    ///
    /// # Arguments
    /// * `ctx` - The shared context, wrapped in an [`Arc`].
    ///
    /// # Returns
    /// The constructed client, or an [`ApiError`] if initialization fails.
    fn from_ctx(ctx: Arc<ClientContext>) -> Result<Self, ApiError>;
}

impl FromContext for DashboardClient {
    fn from_ctx(ctx: Arc<ClientContext>) -> Result<Self, ApiError> {
        DashboardClient::with_config_and_cache(ctx.config.clone(), ctx.cache.clone())
    }
}

impl FromContext for InputUserClient {
    fn from_ctx(ctx: Arc<ClientContext>) -> Result<Self, ApiError> {
        InputUserClient::with_config_and_cache(ctx.config.clone(), ctx.cache.clone())
    }
}

impl FromContext for InputDataClient {
    fn from_ctx(ctx: Arc<ClientContext>) -> Result<Self, ApiError> {
        InputDataClient::with_config_and_cache(ctx.config.clone(), ctx.cache.clone())
    }
}

impl FromContext for PicClient {
    fn from_ctx(ctx: Arc<ClientContext>) -> Result<Self, ApiError> {
        PicClient::with_config_and_cache(ctx.config.clone(), ctx.cache.clone())
    }
}

impl FromContext for UsersClient {
    fn from_ctx(ctx: Arc<ClientContext>) -> Result<Self, ApiError> {
        UsersClient::with_config_and_cache(ctx.config.clone(), ctx.cache.clone())
    }
}

/// Main client for interacting with the CekUnit API.
///
/// This is the primary entry point for all API operations. It manages authentication,
/// session caching, and provides methods to access specialized sub-clients for different
/// parts of the application.
///
/// # Example
/// ```
/// # use cekunit_client::CekUnitClient;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut client = CekUnitClient::new()?;
///
/// // Login and obtain a session
/// let session = client.login()?;
/// println!("Logged in at: {}", session.timestamp);
///
/// // Access the dashboard client
/// let dashboard = client.dashboard()?;
/// let html = dashboard.get_dashboard(Some(1), None, None, None)?;
///
/// // Logout when done
/// client.logout()?;
/// # Ok(())
/// # }
/// ```
pub struct CekUnitClient {
    /// Shared context containing configuration and cache.
    ctx: Arc<ClientContext>,
    /// Client for login operations.
    auth_client: LoginClient,
    /// Client for logout operations.
    logout_client: LogoutClient,
}

impl CekUnitClient {
    /// Creates a new CekUnit client with default configuration loaded from environment variables.
    ///
    /// This constructor loads the configuration using [`EnvConfig::load()`] and initializes
    /// the cache manager and HTTP clients. It returns an error if environment variables
    /// are missing or invalid.
    ///
    /// # Errors
    /// Returns [`ApiError`] if:
    /// - Environment configuration cannot be loaded (see [`EnvError`]).
    /// - HTTP client construction fails.
    /// - Cache directory cannot be created.
    pub fn new() -> Result<Self, ApiError> {
        let auth = LoginClient::new()?;
        let logout = LogoutClient::with_config(auth.config.clone())?;
        let ctx = Arc::new(ClientContext {
            config: auth.config.clone(),
            cache: auth.cache_manager().clone(),
        });
        Ok(Self {
            ctx,
            auth_client: auth,
            logout_client: logout,
        })
    }

    /// Performs login using the credentials from the configuration.
    ///
    /// This method delegates to [`LoginClient::login()`] and stores the session in the cache.
    /// After a successful login, the session cache is available for all sub-clients.
    ///
    /// # Returns
    /// The cached session data ([`CacheData`]) on success.
    ///
    /// # Errors
    /// Returns [`ApiError`] if login fails (invalid credentials, network issues, CSRF token missing).
    pub fn login(&mut self) -> Result<CacheData, ApiError> {
        self.auth_client.login()
    }

    /// Performs logout and clears the session cache.
    ///
    /// This method attempts to log out using the following strategy:
    /// 1. If a dashboard client can be created, it tries to fetch a fresh CSRF token and uses it.
    /// 2. Falls back to a normal logout using the cached token.
    ///
    /// After a successful logout, the session cache is cleared.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the logout request fails after retries.
    pub fn logout(&mut self) -> Result<(), ApiError> {
        if let Ok(dashboard) = self.dashboard() {
            if let Ok(token) = dashboard.get_csrf_token() {
                if self.logout_client.logout_with_token(&token).is_ok() {
                    return Ok(());
                }
            }
        }
        self.logout_client.logout()
    }

    /// Checks if there is an active session in the cache.
    ///
    /// # Returns
    /// - `Ok(Some(CacheData))` if a valid session exists.
    /// - `Ok(None)` if no session is cached or the cached session is invalid.
    /// - `Err(ApiError)` if the cache cannot be read.
    pub fn check_session(&self) -> Result<Option<CacheData>, ApiError> {
        self.auth_client.get_cached_session()
    }

    /// Returns the path to the session cache file.
    pub fn cache_file_path(&self) -> PathBuf {
        self.auth_client.cache_file_path()
    }

    /// Helper method to create a sub-client from the shared context.
    fn make<T>(&self) -> Result<T, ApiError>
    where
        T: FromContext,
    {
        T::from_ctx(self.ctx.clone())
    }

    /// Returns a client for dashboard operations.
    ///
    /// The returned [`DashboardClient`] shares the same configuration and session cache
    /// as the main client.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the client cannot be constructed (unlikely, but possible if
    /// the cache manager fails to clone).
    pub fn dashboard(&self) -> Result<DashboardClient, ApiError> {
        self.make()
    }

    /// Returns a client for input user operations.
    ///
    /// The returned [`InputUserClient`] shares the same configuration and session cache.
    ///
    /// # Errors
    /// Same as [`dashboard`](Self::dashboard).
    pub fn input_user(&self) -> Result<InputUserClient, ApiError> {
        self.make()
    }

    /// Returns a client for input data operations.
    ///
    /// The returned [`InputDataClient`] shares the same configuration and session cache.
    ///
    /// # Errors
    /// Same as [`dashboard`](Self::dashboard).
    pub fn input_data(&self) -> Result<InputDataClient, ApiError> {
        self.make()
    }

    /// Returns a client for PIC (Person In Charge) operations.
    ///
    /// The returned [`PicClient`] shares the same configuration and session cache.
    ///
    /// # Errors
    /// Same as [`dashboard`](Self::dashboard).
    pub fn pic(&self) -> Result<PicClient, ApiError> {
        self.make()
    }

    /// Returns a client for users management operations.
    ///
    /// The returned [`UsersClient`] shares the same configuration and session cache.
    ///
    /// # Errors
    /// Same as [`dashboard`](Self::dashboard).
    pub fn users(&self) -> Result<UsersClient, ApiError> {
        self.make()
    }

    /// Returns a reference to the underlying login client.
    pub fn auth_client(&self) -> &LoginClient {
        &self.auth_client
    }

    /// Returns a reference to the underlying logout client.
    pub fn logout_client(&self) -> &LogoutClient {
        &self.logout_client
    }
}
