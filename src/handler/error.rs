use crate::handler::env::EnvError;
use reqwest;
use serde_json;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(Box<dyn std::error::Error + Send + Sync>),

    #[error("Login failed: {0}")]
    LoginFailed(String),

    #[error("Logout failed: {0}")]
    LogoutFailed(String),

    #[error("CSRF token not found")]
    CsrfTokenNotFound,

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Environment error: {0}")]
    EnvError(#[from] EnvError),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Not authenticated")]
    NotAuthenticated,
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::RequestFailed(Box::new(err))
    }
}
