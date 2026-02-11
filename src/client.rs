use crate::api::auth::utils::cache::{CacheData, CacheManager};
use crate::api::auth::{LoginClient, LogoutClient};
use crate::api::dashboard::{DashboardClient, InputDataClient, InputUserClient, PicClient};
use crate::handler::env::EnvConfig;
use crate::handler::error::ApiError;
use std::path::PathBuf;
use std::sync::Arc;
pub type ConfigType = EnvConfig;
pub type CacheManagerType = CacheManager;
#[derive(Clone)]
pub struct ClientContext {
    pub config: ConfigType,
    pub cache: CacheManagerType,
}
pub trait FromContext: Sized {
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
pub struct CekUnitClient {
    ctx: Arc<ClientContext>,
    auth_client: LoginClient,
    logout_client: LogoutClient,
}
impl CekUnitClient {
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
    pub fn login(&mut self) -> Result<CacheData, ApiError> {
        self.auth_client.login()
    }
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
    pub fn check_session(&self) -> Result<Option<CacheData>, ApiError> {
        self.auth_client.get_cached_session()
    }
    pub fn cache_file_path(&self) -> PathBuf {
        self.auth_client.cache_file_path()
    }
    fn make<T>(&self) -> Result<T, ApiError>
    where
        T: FromContext,
    {
        T::from_ctx(self.ctx.clone())
    }
    pub fn dashboard(&self) -> Result<DashboardClient, ApiError> {
        self.make()
    }
    pub fn input_user(&self) -> Result<InputUserClient, ApiError> {
        self.make()
    }
    pub fn input_data(&self) -> Result<InputDataClient, ApiError> {
        self.make()
    }
    pub fn pic(&self) -> Result<PicClient, ApiError> {
        self.make()
    }
    pub fn auth_client(&self) -> &LoginClient {
        &self.auth_client
    }
    pub fn logout_client(&self) -> &LogoutClient {
        &self.logout_client
    }
}
