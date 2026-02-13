//! Environment configuration management.
//!
//! This module provides functionality to load and validate configuration from environment
//! variables, as well as constructing full URLs for various API endpoints.
//!
//! The primary types are:
//! - [`EnvError`]: Errors that can occur during environment loading.
//! - [`EnvConfig`]: Holds all configuration values and provides methods to build endpoint URLs.

use std::env;
use thiserror::Error;

/// Errors that can occur while loading or validating environment variables.
#[derive(Debug, Error, Clone)]
pub enum EnvError {
    /// A required environment variable is missing.
    #[error("Environment variable '{0}' not found")]
    NotFound(String),

    /// A required environment variable is set but empty.
    #[error("'{0}' must not be empty")]
    Empty(String),

    /// A value is invalid (e.g., email format, password length).
    #[error("'{0}' is invalid: {1}")]
    Invalid(String, String),

    /// A URL has an invalid scheme (must be http or https).
    #[error("Invalid URL format for '{0}': {1}")]
    InvalidUrl(String, String),

    /// An endpoint path contains illegal characters (currently unused but reserved).
    #[error("Endpoint '{0}' contains illegal characters: {1}")]
    InvalidEndpoint(String, String),
}

/// Configuration loaded from environment variables.
///
/// All fields are required and must pass validation.
/// Use [`EnvConfig::load()`] to create an instance.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// Email address used for authentication (must contain '@').
    pub user_email: String,
    /// Password used for authentication (minimum length 8).
    pub user_password: String,
    /// Base URL of the application (must start with http:// or https://).
    pub base_url: String,
    /// Endpoint path for login.
    pub login_endpoint: String,
    /// Endpoint path for logout.
    pub logout_endpoint: String,
    /// Endpoint path for the main dashboard.
    pub dashboard_endpoint: String,
    /// Endpoint path for exporting CekUnit data.
    pub cekunit_export_endpoint: String,
    /// Endpoint path for fetching unique column values in CekUnit.
    pub cekunit_unique_endpoint: String,
    /// Endpoint path for deleting CekUnit records by category.
    pub cekunit_delete_category_endpoint: String,
    /// Endpoint path for deleting all CekUnit records.
    pub delete_all_endpoint: String,
    /// Endpoint path template for individual CekUnit items (will have ID appended).
    pub cekunit_item_endpoint: String,
    /// Endpoint path for input user listing/management.
    pub input_user_endpoint: String,
    /// Endpoint path for exporting input user data.
    pub input_user_export_endpoint: String,
    /// Endpoint path for input data forms.
    pub input_data_endpoint: String,
    /// Endpoint path for PIC (Person In Charge) listing.
    pub pic_endpoint: String,
    /// Endpoint path for creating a new PIC.
    pub input_pic_endpoint: String,
    /// Endpoint path template for individual PIC items (will have ID appended).
    pub pic_item_endpoint: String,
    /// Endpoint path for users listing.
    pub users_endpoint: String,
    /// Endpoint path template for individual user items (will have ID appended).
    pub users_item_endpoint: String,
}

impl EnvConfig {
    /// Loads and validates configuration from environment variables.
    ///
    /// This function reads the `.env` file (if present) using `dotenv`, then reads
    /// the required environment variables. All fields are mandatory and validated.
    ///
    /// # Returns
    /// - `Ok(EnvConfig)` if all variables are present and valid.
    /// - `Err(EnvError)` otherwise.
    ///
    /// # Example
    /// ```
    /// # use your_crate::handler::env::EnvConfig;
    /// match EnvConfig::load() {
    ///     Ok(config) => println!("Base URL: {}", config.base_url),
    ///     Err(e) => eprintln!("Config error: {}", e),
    /// }
    /// ```
    pub fn load() -> Result<Self, EnvError> {
        dotenv::dotenv().ok();
        let config = Self {
            user_email: get_env_non_empty("USER_EMAIL")?,
            user_password: get_env_non_empty("USER_PASSWORD")?,
            base_url: get_env_url("BASE_URL")?,
            login_endpoint: get_env_endpoint("LOGIN_ENDPOINT")?,
            logout_endpoint: get_env_endpoint("LOGOUT_ENDPOINT")?,
            dashboard_endpoint: get_env_endpoint("DASHBOARD_ENDPOINT")?,
            cekunit_export_endpoint: get_env_endpoint("CEKUNIT_EXPORT_ENDPOINT")?,
            cekunit_unique_endpoint: get_env_endpoint("CEKUNIT_UNIQUE_ENDPOINT")?,
            cekunit_delete_category_endpoint: get_env_endpoint("CEKUNIT_DELETE_CATEGORY_ENDPOINT")?,
            delete_all_endpoint: get_env_endpoint("DELETE_ALL_ENDPOINT")?,
            cekunit_item_endpoint: get_env_endpoint("CEKUNIT_ITEM_ENDPOINT")?,
            input_user_endpoint: get_env_endpoint("INPUT_USER_ENDPOINT")?,
            input_user_export_endpoint: get_env_endpoint("INPUT_USER_EXPORT_ENDPOINT")?,
            input_data_endpoint: get_env_endpoint("INPUT_DATA_ENDPOINT")?,
            pic_endpoint: get_env_endpoint("PIC_ENDPOINT")?,
            input_pic_endpoint: get_env_endpoint("INPUT_PIC_ENDPOINT")?,
            pic_item_endpoint: get_env_endpoint("PIC_ITEM_ENDPOINT")?,
            users_endpoint: get_env_endpoint("USERS_ENDPOINT")?,
            users_item_endpoint: get_env_endpoint("USERS_ITEM_ENDPOINT")?,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validates the loaded configuration values.
    ///
    /// Checks:
    /// - `user_email` contains an '@' character.
    /// - `user_password` is at least 8 characters long.
    /// - `base_url` starts with "http://" or "https://".
    ///
    /// # Returns
    /// - `Ok(())` if all checks pass.
    /// - `Err(EnvError::Invalid)` otherwise.
    pub fn validate(&self) -> Result<(), EnvError> {
        if !self.user_email.contains('@') {
            return Err(EnvError::Invalid(
                "USER_EMAIL".into(),
                "must contain '@' character".into(),
            ));
        }
        if self.user_password.len() < 8 {
            return Err(EnvError::Invalid(
                "USER_PASSWORD".into(),
                "must be at least 8 characters".into(),
            ));
        }
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(EnvError::InvalidUrl(
                "BASE_URL".into(),
                "must start with http:// or https://".into(),
            ));
        }
        Ok(())
    }

    /// Builds a full URL by concatenating the base URL with the given endpoint.
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}/{}", self.base_url, endpoint)
    }

    /// Returns the full login URL.
    pub fn full_login_url(&self) -> String {
        self.build_url(&self.login_endpoint)
    }

    /// Returns the full logout URL.
    pub fn full_logout_url(&self) -> String {
        self.build_url(&self.logout_endpoint)
    }

    /// Returns the full dashboard URL.
    pub fn full_dashboard_url(&self) -> String {
        self.build_url(&self.dashboard_endpoint)
    }

    /// Returns the full URL for exporting CekUnit data.
    pub fn full_cekunit_export_url(&self) -> String {
        self.build_url(&self.cekunit_export_endpoint)
    }

    /// Returns the full URL for fetching unique column values.
    pub fn full_cekunit_unique_url(&self) -> String {
        self.build_url(&self.cekunit_unique_endpoint)
    }

    /// Returns the full URL for deleting CekUnit records by category.
    pub fn full_cekunit_delete_category_url(&self) -> String {
        self.build_url(&self.cekunit_delete_category_endpoint)
    }

    /// Returns the full URL for deleting all CekUnit records.
    pub fn full_delete_all_url(&self) -> String {
        self.build_url(&self.delete_all_endpoint)
    }

    /// Returns the full URL for a specific CekUnit item.
    ///
    /// # Arguments
    /// * `no` - The item identifier to append to the endpoint.
    pub fn full_cekunit_item_url(&self, no: &str) -> String {
        format!("{}/{}", self.build_url(&self.cekunit_item_endpoint), no)
    }

    /// Returns the full URL for input user listing.
    pub fn full_input_user_url(&self) -> String {
        self.build_url(&self.input_user_endpoint)
    }

    /// Returns the full URL for exporting input user data.
    pub fn full_input_user_export_url(&self) -> String {
        self.build_url(&self.input_user_export_endpoint)
    }

    /// Returns the full URL for input data forms.
    pub fn full_input_data_url(&self) -> String {
        self.build_url(&self.input_data_endpoint)
    }

    /// Returns the full URL for PIC listing.
    pub fn full_pic_url(&self) -> String {
        self.build_url(&self.pic_endpoint)
    }

    /// Returns the full URL for creating a new PIC.
    pub fn full_input_pic_url(&self) -> String {
        self.build_url(&self.input_pic_endpoint)
    }

    /// Returns the full URL for a specific PIC item.
    ///
    /// # Arguments
    /// * `id` - The item identifier to append to the endpoint.
    pub fn full_pic_item_url(&self, id: &str) -> String {
        format!("{}/{}", self.build_url(&self.pic_item_endpoint), id)
    }

    /// Returns the full URL for users listing.
    pub fn full_users_url(&self) -> String {
        self.build_url(&self.users_endpoint)
    }

    /// Returns the full URL for a specific user item.
    ///
    /// # Arguments
    /// * `id` - The item identifier to append to the endpoint.
    pub fn full_users_item_url(&self, id: &str) -> String {
        format!("{}/{}", self.build_url(&self.users_item_endpoint), id)
    }
}

/// Retrieves a non-empty environment variable.
///
/// # Arguments
/// * `key` - Name of the environment variable.
///
/// # Returns
/// - `Ok(String)` with the trimmed value if present and non-empty.
/// - `Err(EnvError::NotFound)` if the variable is not set.
/// - `Err(EnvError::Empty)` if the variable is set but empty after trimming.
fn get_env_non_empty(key: &str) -> Result<String, EnvError> {
    let val = env::var(key).map_err(|_| EnvError::NotFound(key.to_string()))?;
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return Err(EnvError::Empty(key.to_string()));
    }
    Ok(trimmed.to_string())
}

/// Retrieves and validates a URL environment variable.
///
/// # Arguments
/// * `key` - Name of the environment variable.
///
/// # Returns
/// - `Ok(String)` with the normalized URL if it starts with http:// or https://.
/// - `Err(EnvError::InvalidUrl)` otherwise, or any error from `get_env_non_empty`.
fn get_env_url(key: &str) -> Result<String, EnvError> {
    let val = get_env_non_empty(key)?;
    if !val.starts_with("http://") && !val.starts_with("https://") {
        return Err(EnvError::InvalidUrl(
            key.to_string(),
            "must start with http:// or https://".into(),
        ));
    }
    Ok(normalize_base(val))
}

/// Retrieves and normalizes an endpoint path environment variable.
///
/// The endpoint is trimmed and leading slashes are removed.
///
/// # Arguments
/// * `key` - Name of the environment variable.
///
/// # Returns
/// `Ok(String)` with the normalized endpoint, or any error from `get_env_non_empty`.
fn get_env_endpoint(key: &str) -> Result<String, EnvError> {
    let val = get_env_non_empty(key)?;
    Ok(normalize_endpoint(val))
}

/// Normalizes a base URL by trimming and removing a trailing slash if present.
fn normalize_base(mut base: String) -> String {
    base = base.trim().to_string();
    if base.ends_with('/') {
        base.pop();
    }
    base
}

/// Normalizes an endpoint path by trimming and removing leading slashes.
fn normalize_endpoint(mut endpoint: String) -> String {
    endpoint = endpoint.trim().to_string();
    endpoint = endpoint.trim_start_matches('/').to_string();
    endpoint
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// Safely removes an environment variable.
    fn safe_remove_var(key: &str) {
        unsafe { env::remove_var(key) }
    }

    /// Safely sets an environment variable.
    fn safe_set_var(key: &str, value: &str) {
        unsafe { env::set_var(key, value) }
    }

    /// Resets all environment variables used in tests.
    fn setup() {
        safe_remove_var("USER_EMAIL");
        safe_remove_var("USER_PASSWORD");
        safe_remove_var("BASE_URL");
        safe_remove_var("LOGIN_ENDPOINT");
        safe_remove_var("LOGOUT_ENDPOINT");
        safe_remove_var("DASHBOARD_ENDPOINT");
        safe_remove_var("CEKUNIT_EXPORT_ENDPOINT");
        safe_remove_var("CEKUNIT_UNIQUE_ENDPOINT");
        safe_remove_var("CEKUNIT_DELETE_CATEGORY_ENDPOINT");
        safe_remove_var("DELETE_ALL_ENDPOINT");
        safe_remove_var("CEKUNIT_ITEM_ENDPOINT");
        safe_remove_var("INPUT_USER_ENDPOINT");
        safe_remove_var("INPUT_USER_EXPORT_ENDPOINT");
        safe_remove_var("INPUT_DATA_ENDPOINT");
        safe_remove_var("PIC_ENDPOINT");
        safe_remove_var("INPUT_PIC_ENDPOINT");
        safe_remove_var("PIC_ITEM_ENDPOINT");
    }

    #[test]
    fn test_missing_var() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "password123");
        safe_set_var("BASE_URL", "http://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::NotFound(_))));
    }

    #[test]
    fn test_empty_var() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "password123");
        safe_set_var("BASE_URL", "http://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        safe_set_var("DASHBOARD_ENDPOINT", "");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::Empty(_))));
    }

    #[test]
    fn test_invalid_url() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "password123");
        safe_set_var("BASE_URL", "ftp://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        safe_set_var("DASHBOARD_ENDPOINT", "dashboard");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::InvalidUrl(_, _))));
    }

    #[test]
    fn test_password_too_short() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "123");
        safe_set_var("BASE_URL", "http://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        safe_set_var("DASHBOARD_ENDPOINT", "dashboard");
        safe_set_var("CEKUNIT_EXPORT_ENDPOINT", "export");
        safe_set_var("CEKUNIT_UNIQUE_ENDPOINT", "unique");
        safe_set_var("CEKUNIT_DELETE_CATEGORY_ENDPOINT", "delete_cat");
        safe_set_var("DELETE_ALL_ENDPOINT", "delete_all");
        safe_set_var("CEKUNIT_ITEM_ENDPOINT", "item");
        safe_set_var("INPUT_USER_ENDPOINT", "input_user");
        safe_set_var("INPUT_USER_EXPORT_ENDPOINT", "input_user_export");
        safe_set_var("INPUT_DATA_ENDPOINT", "input_data");
        safe_set_var("PIC_ENDPOINT", "pic");
        safe_set_var("INPUT_PIC_ENDPOINT", "input_pic");
        safe_set_var("PIC_ITEM_ENDPOINT", "pic_item");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::Invalid(_, _))));
    }
}
