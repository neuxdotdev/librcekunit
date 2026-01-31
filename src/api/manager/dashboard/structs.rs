use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NasabahData {
    pub no: u32,
    pub no_perjanjian: String,
    pub nama_nasabah: String,
    pub nopol: String,
    pub coll: String,
    pub pic: String,
    pub kategori: String,
    pub jto: String,
    pub no_rangka: String,
    pub no_mesin: String,
    pub merk: String,
    pub type_unit: String,
    pub warna: String,
    pub status: String,
    pub actual_penyelesaian: String,
    pub angsuran_ke: String,
    pub tenor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub current_page: u32,
    pub total_pages: u32,
    pub total_data: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub data: Vec<NasabahData>,
    pub pagination: PaginationInfo,
    pub user_info: UserInfo,
}

#[derive(Debug)]
pub enum DashboardError {
    Request(String),
    Parse(String),
    Cache(String),
    Json(String),
    NotAuthenticated,
}

impl std::fmt::Display for DashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DashboardError::Request(msg) => write!(f, "Request error: {}", msg),
            DashboardError::Parse(msg) => write!(f, "Parse error: {}", msg),
            DashboardError::Cache(msg) => write!(f, "Cache error: {}", msg),
            DashboardError::Json(msg) => write!(f, "JSON error: {}", msg),
            DashboardError::NotAuthenticated => write!(f, "Not authenticated"),
        }
    }
}

impl std::error::Error for DashboardError {}

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query: String,
    pub column: Option<String>,
    pub sort_column: Option<String>,
    pub sort_direction: Option<String>,
    pub page: Option<u32>,
}
