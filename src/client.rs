use crate::api::auth::utils::cache::CacheData;
use crate::api::auth::{LoginClient, LogoutClient};
use crate::handler::error::ApiError;
use std::path::PathBuf;
pub struct CekUnitClient {
    pub auth_client: LoginClient,
    pub logout_client: LogoutClient,
}
impl CekUnitClient {
    pub fn new() -> Result<Self, ApiError> {
        Ok(Self {
            auth_client: LoginClient::new()?,
            logout_client: LogoutClient::new()?,
        })
    }
    pub fn login(&mut self) -> Result<CacheData, ApiError> {
        self.auth_client.login()
    }
    pub fn logout(&mut self) -> Result<(), ApiError> {
        self.logout_client.logout()
    }
    pub fn check_session(&self) -> Result<Option<CacheData>, ApiError> {
        self.auth_client.get_cached_session()
    }
    pub fn cache_file_path(&self) -> PathBuf {
        self.auth_client.cache_file_path()
    }
    pub fn auth_client(&self) -> &LoginClient {
        &self.auth_client
    }
    pub fn logout_client(&self) -> &LogoutClient {
        &self.logout_client
    }
}
