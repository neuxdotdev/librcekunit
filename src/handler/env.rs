use std::env;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum EnvError {
    #[error("Environment variable {0} not found")]
    NotFound(String),
    #[error("Invalid environment variable: {0}")]
    Invalid(String),
}
#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub user_email: String,
    pub user_password: String,
    pub base_url: String,
    pub login_endpoint: String,
    pub logout_endpoint: String,
}
impl EnvConfig {
    pub fn load() -> Result<Self, EnvError> {
        dotenv::dotenv().ok();
        Ok(Self {
            user_email: get_env("USER_EMAIL")?,
            user_password: get_env("USER_PASSWORD")?,
            base_url: get_env("BASE_URL")?,
            login_endpoint: get_env("LOGIN_ENDPOINT")?,
            logout_endpoint: get_env("LOGOUT_ENDPOINT")?,
        })
    }
    pub fn from_values(
        user_email: String,
        user_password: String,
        base_url: String,
        login_endpoint: String,
        logout_endpoint: String,
    ) -> Self {
        Self {
            user_email,
            user_password,
            base_url,
            login_endpoint,
            logout_endpoint,
        }
    }
    pub fn full_login_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        let endpoint = self.login_endpoint.trim_start_matches('/');
        format!("{}/{}", base, endpoint)
    }
    pub fn full_logout_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        let endpoint = self.logout_endpoint.trim_start_matches('/');
        format!("{}/{}", base, endpoint)
    }
}
fn get_env(key: &str) -> Result<String, EnvError> {
    env::var(key).map_err(|_| EnvError::NotFound(key.to_string()))
}
