use crate::handler::env::EnvError;
use reqwest::StatusCode;
use serde_json;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ApiError {
    #[error(" HTTP request failed: {0}")]
    RequestFailed(String),
    #[error(" Request timeout")]
    RequestTimeout,
    #[error(" Login failed: {0}")]
    LoginFailed(String),
    #[error(" Logout failed: {0}")]
    LogoutFailed(String),
    #[error(" Not authenticated – please login first")]
    NotAuthenticated,
    #[error("️ CSRF token not found in HTML")]
    CsrfTokenNotFound,
    #[error("️ CSRF token expired or invalid (HTTP 419)")]
    CsrfExpired,
    #[error("️ CSRF token validation failed: {0}")]
    CsrfInvalid(String),
    #[error("️ Validation error (HTTP 422): {0}")]
    ValidationError(String),
    #[error("️ Too many requests (HTTP 429) – please try later")]
    TooManyRequests,
    #[error(" Resource not found (HTTP 404)")]
    ResourceNotFound,
    #[error(" Unauthorized (HTTP 401) – session expired?")]
    Unauthorized,
    #[error(" Forbidden (HTTP 403) – insufficient permissions")]
    Forbidden,
    #[error(" Server error (HTTP {0})")]
    ServerError(u16),
    #[error(" Cache error: {0}")]
    CacheError(String),
    #[error("️ Environment error: {0}")]
    EnvError(#[from] EnvError),
    #[error(" JSON parsing error: {0}")]
    JsonError(String),
    #[error(" IO error: {0}")]
    IoError(String),
    #[error(" HTML parsing error: {0}")]
    HtmlParseError(String),
    #[error(" {0}")]
    Other(String),
}

impl ApiError {
    pub fn from_status(status: StatusCode, body: Option<&str>) -> Self {
        let body_preview = body.unwrap_or("").split('<').next().unwrap_or("").trim();
        match status.as_u16() {
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::ResourceNotFound,
            419 => Self::CsrfExpired,
            422 => Self::ValidationError(body_preview.to_string()),
            429 => Self::TooManyRequests,
            500..=599 => Self::ServerError(status.as_u16()),
            _ => Self::RequestFailed(format!("HTTP {}: {}", status, body_preview)),
        }
    }

    pub fn from_reqwest_error(err: reqwest::Error, context: &str) -> Self {
        if err.is_timeout() {
            return Self::RequestTimeout;
        }
        if err.is_connect() {
            return Self::RequestFailed(format!("Connection failed ({}): {}", context, err));
        }
        Self::RequestFailed(format!("{}: {}", context, err))
    }

    pub fn csrf_not_found() -> Self {
        Self::CsrfTokenNotFound
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        Self::from_reqwest_error(err, "reqwest")
    }
}

impl From<reqwest::header::InvalidHeaderValue> for ApiError {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Self::CacheError(format!("Invalid header value: {}", err))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<String> for ApiError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

impl From<&str> for ApiError {
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}
