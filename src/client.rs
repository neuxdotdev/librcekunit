use crate::api::auth::utils::cache::CacheData;
use crate::api::auth::{LoginClient, LogoutClient};
use crate::api::manager::DashboardClient;
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

    // Dashboard-related methods
    pub fn get_dashboard_client(&self) -> Result<DashboardClient, ApiError> {
        let cache_data = self.auth_client.get_cached_session()?;
        let config = self.auth_client.config().clone();

        DashboardClient::new(config.base_url, cache_data)
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))
    }

    pub fn fetch_dashboard(&self, page: Option<u32>) -> Result<crate::DashboardData, ApiError> {
        let dashboard_client = self.get_dashboard_client()?;
        dashboard_client
            .fetch_dashboard(page)
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))
    }

    pub fn search_data(
        &self,
        query: &str,
        column: Option<&str>,
        page: Option<u32>,
    ) -> Result<crate::DashboardData, ApiError> {
        let dashboard_client = self.get_dashboard_client()?;
        let params = crate::SearchParams {
            query: query.to_string(),
            column: column.map(|s| s.to_string()),
            sort_column: None,
            sort_direction: None,
            page,
        };

        dashboard_client
            .search_data(&params)
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))
    }

    pub fn export_data(&self, format: &str) -> Result<Vec<u8>, ApiError> {
        let dashboard_client = self.get_dashboard_client()?;
        dashboard_client
            .export_data(format)
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))
    }

    pub fn get_unique_values(&self, column: &str) -> Result<Vec<String>, ApiError> {
        let dashboard_client = self.get_dashboard_client()?;
        dashboard_client
            .get_unique_values(column)
            .map_err(|e| ApiError::RequestFailed(Box::new(e)))
    }
}
