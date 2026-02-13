//! # CekUnit Client Library
//!
//! A comprehensive Rust client for interacting with the CekUnit web application.
//! This library provides a type-safe, ergonomic API for authentication and all
//! major features of the CekUnit platform, including dashboard management,
//! input data (nasabah) operations, PIC (Person In Charge) management,
//! user management, and export functionality.
//!
//! ## Overview
//!
//! The library is structured around a main client [`CekUnitClient`] that manages
//! a shared session cache and provides access to specialized sub‑clients for
//! different parts of the application. All network operations are performed via
//! a blocking `reqwest` client with configurable timeouts, retries, and connection
//! pooling.
//!
//! ## Features
//!
//! - **Authentication**: Login with email/password, automatic CSRF token handling,
//!   session persistence via filesystem cache.
//! - **Dashboard**: Fetch paginated CekUnit lists, export data (Excel, PDF, CSV),
//!   get unique column values, delete records (single, by category, or all).
//! - **Input Data (Nasabah)**: Submit new customer records.
//! - **Input User**: List and export user‑input data with search, sort, and date filters.
//! - **PIC Management**: Create, update, delete, and list Persons In Charge.
//! - **User Management**: List and update application users.
//!
//! ## Caching
//!
//! Upon successful login, session cookies and the current CSRF token are stored
//! in a JSON file inside the system’s cache directory (e.g., `~/.cache/cekunit/` on Linux).
//! All subsequent requests automatically attach these cookies, so you only need
//! to log in once per session.
//!
//! ## Environment Configuration
//!
//! The library reads configuration from environment variables (or a `.env` file).
//! Required variables include:
//!
//! - `USER_EMAIL` – login email
//! - `USER_PASSWORD` – login password
//! - `BASE_URL` – base URL of the CekUnit installation (e.g., `https://example.com`)
//! - Various endpoint variables (see [`EnvConfig`] documentation for the full list)
//!
//! ## Example
//!
//! ```no_run
//! use cekunit_client::CekUnitClient;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create the main client – loads configuration from environment
//!     let mut client = CekUnitClient::new()?;
//!
//!     // Log in (if no valid session exists, this will perform a new login)
//!     let session = client.login()?;
//!     println!("Logged in at: {}", session.timestamp);
//!
//!     // Access the dashboard client
//!     let dashboard = client.dashboard()?;
//!     let html = dashboard.get_dashboard(Some(1), None, Some("created_at"), Some("desc"))?;
//!
//!     // Export data as Excel
//!     let excel_data = dashboard.export_cekunit("excel", "created_at", "desc")?;
//!     std::fs::write("export.xlsx", excel_data)?;
//!
//!     // Log out when done
//!     client.logout()?;
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod client;
pub mod handler;

// Re‑export public API for easy access
pub use crate::api::auth::loging::LoginClient;
pub use crate::api::auth::logout::LogoutClient;
pub use crate::api::auth::utils::cache::{CacheData, CacheManager};
pub use crate::api::dashboard::{
    DashboardClient, InputDataClient, InputUserClient, PicClient, UsersClient,
};
pub use crate::client::CekUnitClient;
pub use crate::handler::env::EnvConfig;
pub use crate::handler::error::ApiError;

/// Utility functions and types for internal use, but exposed for advanced scenarios.
///
/// This module re‑exports lower‑level components from `api::auth::utils` that may be
/// useful for custom integrations or testing.
pub mod utils {
    pub use crate::api::auth::utils::cache::{CacheManager, Cookie};
    pub use crate::api::auth::utils::cookies;
    pub use crate::api::auth::utils::token;
}

/// Returns the current crate version as defined in `Cargo.toml`.
///
/// # Example
/// ```
/// use cekunit_client::version;
/// println!("Client version: {}", version());
/// ```
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Returns the current crate name as defined in `Cargo.toml`.
///
/// # Example
/// ```
/// use cekunit_client::name;
/// println!("Crate name: {}", name());
/// ```
pub fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/// Returns a [`BuildInfo`] struct containing metadata about the crate.
///
/// # Example
/// ```
/// use cekunit_client::build_info;
/// let info = build_info();
/// println!("{}", info);
/// ```
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: version(),
        name: name(),
        authors: env!("CARGO_PKG_AUTHORS"),
        description: env!("CARGO_PKG_DESCRIPTION"),
        repository: env!("CARGO_PKG_REPOSITORY"),
    }
}

/// Build metadata for the CekUnit client crate.
///
/// This struct holds static strings obtained from Cargo environment variables
/// at compile time. It can be used for logging, diagnostics, or display purposes.
#[derive(Debug, Clone)]
pub struct BuildInfo {
    /// The crate version (e.g., `"0.1.0"`).
    pub version: &'static str,
    /// The crate name (e.g., `"cekunit_client"`).
    pub name: &'static str,
    /// The authors string from `Cargo.toml`.
    pub authors: &'static str,
    /// The description from `Cargo.toml`.
    pub description: &'static str,
    /// The repository URL from `Cargo.toml`.
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
