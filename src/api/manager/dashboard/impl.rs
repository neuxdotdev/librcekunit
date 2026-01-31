use super::fetch::DashboardClient;
use super::struck::{DashboardData, DashboardError, SearchParams};
use crate::api::auth::utils::cache::CacheData;

impl DashboardClient {
    /// Get dashboard data with default page (1)
    pub fn get_dashboard(&self) -> Result<DashboardData, DashboardError> {
        self.fetch_dashboard(Some(1))
    }

    /// Simple search by query only
    pub fn simple_search(&self, query: &str) -> Result<DashboardData, DashboardError> {
        let params = SearchParams {
            query: query.to_string(),
            column: None,
            sort_column: None,
            sort_direction: None,
            page: None,
        };
        self.search_data(&params)
    }

    /// Search with column specification
    pub fn search_by_column(&self, query: &str, column: &str) -> Result<DashboardData, DashboardError> {
        let params = SearchParams {
            query: query.to_string(),
            column: Some(column.to_string()),
            sort_column: None,
            sort_direction: None,
            page: None,
        };
        self.search_data(&params)
    }

    /// Get paginated data
    pub fn get_page(&self, page: u32) -> Result<DashboardData, DashboardError> {
        self.fetch_dashboard(Some(page))
    }

    /// Export to CSV
    pub fn export_csv(&self) -> Result<Vec<u8>, DashboardError> {
        self.export_data("csv")
    }

    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.cache_data.is_some()
    }

    /// Get base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get cache data (if any)
    pub fn cache_data(&self) -> Option<&CacheData> {
        self.cache_data.as_ref()
    }

    /// Create search params builder
    pub fn search_builder(&self) -> SearchBuilder {
        SearchBuilder::new()
    }
}

/// Builder pattern untuk search params
pub struct SearchBuilder {
    query: String,
    column: Option<String>,
    sort_column: Option<String>,
    sort_direction: Option<String>,
    page: Option<u32>,
}

impl SearchBuilder {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            column: None,
            sort_column: None,
            sort_direction: None,
            page: None,
        }
    }

    pub fn query(mut self, query: &str) -> Self {
        self.query = query.to_string();
        self
    }

    pub fn column(mut self, column: &str) -> Self {
        self.column = Some(column.to_string());
        self
    }

    pub fn sort_by(mut self, column: &str, direction: &str) -> Self {
        self.sort_column = Some(column.to_string());
        self.sort_direction = Some(direction.to_string());
        self
    }

    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    pub fn build(self) -> SearchParams {
        SearchParams {
            query: self.query,
            column: self.column,
            sort_column: self.sort_column,
            sort_direction: self.sort_direction,
            page: self.page,
        }
    }
}
