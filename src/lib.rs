pub mod api;
pub mod client;
pub mod handler;
pub use crate::api::auth::loging::LoginClient;
pub use crate::api::auth::utils::cache::{CacheData, CacheManager};
pub use crate::api::dashboard::{DashboardClient, InputDataClient, InputUserClient, PicClient};
pub use api::auth::logout::LogoutClient;
pub use client::CekUnitClient;
pub use handler::env::EnvConfig;
pub use handler::error::ApiError;
pub mod utils {
    pub use crate::api::auth::utils::cache::{CacheManager, Cookie};
    pub use crate::api::auth::utils::cookies;
    pub use crate::api::auth::utils::token;
}
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
pub fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: version(),
        name: name(),
        authors: env!("CARGO_PKG_AUTHORS"),
        description: env!("CARGO_PKG_DESCRIPTION"),
        repository: env!("CARGO_PKG_REPOSITORY"),
    }
}
#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub version: &'static str,
    pub name: &'static str,
    pub authors: &'static str,
    pub description: &'static str,
    pub repository: &'static str,
}
impl std::fmt::Display for BuildInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} v{}\n{}\nAuthors: {}\nRepository: {}",
            self.name, self.version, self.description, self.authors, self.repository
        )
    }
}
