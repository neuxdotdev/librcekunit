//! Error types for API operations.
//!
//! This module defines the [`ApiError`] enum, which represents all possible errors
//! that can occur when interacting with the CekUnit API, including network issues,
//! authentication failures, CSRF token problems, and various HTTP status codes.
//!
//! The error type implements [`std::error::Error`] via `thiserror` and provides
//! convenient conversion traits and helper methods for creating errors from
//! common sources.

use crate::handler::env::EnvError;
use reqwest::StatusCode;
use serde_json;
use thiserror::Error;

/// Represents all errors that can occur in the CekUnit API client.
///
/// Each variant carries additional context where appropriate.
#[derive(Debug, Error, Clone)]
pub enum ApiError {
    /// A general HTTP request failed.
    ///
    /// This can happen due to network issues, invalid URLs, or other transport errors.
    /// The string provides details about the failure.
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    /// The request timed out.
    ///
    /// This occurs when the server does not respond within the configured timeout.
    #[error("Request timeout")]
    RequestTimeout,

    /// Login failed.
    ///
    /// The string provides additional context, such as the HTTP status or error message.
    #[error("Login failed: {0}")]
    LoginFailed(String),

    /// Logout failed.
    ///
    /// The string provides additional context, such as the HTTP status or error message.
    #[error("Logout failed: {0}")]
    LogoutFailed(String),

    /// The client is not authenticated.
    ///
    /// This error is returned when an operation requires a valid session, but none exists
    /// or the session has expired.
    #[error("Not authenticated – please login first")]
    NotAuthenticated,

    /// CSRF token could not be found in the HTML response.
    ///
    /// This typically indicates that the login page structure has changed or the
    /// expected input/meta tag is missing.
    #[error("CSRF token not found in HTML")]
    CsrfTokenNotFound,

    /// CSRF token has expired or is invalid (HTTP 419).
    ///
    /// The server returned a 419 status, meaning the token is no longer valid.
    #[error("CSRF token expired or invalid (HTTP 419)")]
    CsrfExpired,

    /// CSRF token validation failed.
    ///
    /// This is a generic validation error for CSRF-related issues, distinct from
    /// the specific HTTP 419 case.
    #[error("CSRF token validation failed: {0}")]
    CsrfInvalid(String),

    /// Validation error (HTTP 422).
    ///
    /// The server returned a 422 status, usually indicating that the submitted data
    /// (e.g., form fields) failed validation. The string contains a preview of the
    /// response body.
    #[error("Validation error (HTTP 422): {0}")]
    ValidationError(String),

    /// Too many requests (HTTP 429).
    ///
    /// The server is rate-limiting the client.
    #[error("Too many requests (HTTP 429) – please try later")]
    TooManyRequests,

    /// Resource not found (HTTP 404).
    #[error("Resource not found (HTTP 404)")]
    ResourceNotFound,

    /// Unauthorized (HTTP 401) – session may have expired.
    #[error("Unauthorized (HTTP 401) – session expired?")]
    Unauthorized,

    /// Forbidden (HTTP 403) – insufficient permissions.
    #[error("Forbidden (HTTP 403) – insufficient permissions")]
    Forbidden,

    /// Server error (HTTP 5xx).
    ///
    /// The server returned a 5xx status code. The inner value is the exact HTTP status.
    #[error("Server error (HTTP {0})")]
    ServerError(u16),

    /// Cache-related error.
    ///
    /// This can occur when reading/writing the session cache file or when
    /// serializing/deserializing cache data.
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Environment error.
    ///
    /// This wraps [`EnvError`] from the environment configuration module.
    #[error("Environment error: {0}")]
    EnvError(#[from] EnvError),

    /// JSON parsing error.
    ///
    /// Occurs when a response is expected to be JSON but cannot be parsed.
    #[error("JSON parsing error: {0}")]
    JsonError(String),

    /// I/O error.
    ///
    /// Wraps [`std::io::Error`] for filesystem or other I/O operations.
    #[error("IO error: {0}")]
    IoError(String),

    /// HTML parsing error.
    ///
    /// This can happen when extracting data from HTML (e.g., CSRF token) fails.
    #[error("HTML parsing error: {0}")]
    HtmlParseError(String),

    /// A catch-all for other errors.
    ///
    /// Used when no more specific variant applies.
    #[error("{0}")]
    Other(String),
}

impl ApiError {
    /// Creates an appropriate [`ApiError`] from an HTTP status code and optional response body.
    ///
    /// This function maps known status codes to specific error variants and provides
    /// a preview of the response body for client errors (422, etc.).
    ///
    /// # Arguments
    /// * `status` - The HTTP status code from the response.
    /// * `body` - Optional response body text. If provided, a preview is used in some variants.
    ///
    /// # Returns
    /// An `ApiError` variant corresponding to the status code.
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

    /// Creates an [`ApiError`] from a [`reqwest::Error`], adding context about the operation.
    ///
    /// This method distinguishes between timeout, connection, and other request errors.
    ///
    /// # Arguments
    /// * `err` - The `reqwest::Error` to convert.
    /// * `context` - A string describing the operation that failed (e.g., "GET users").
    ///
    /// # Returns
    /// An `ApiError` variant that best represents the underlying error.
    pub fn from_reqwest_error(err: reqwest::Error, context: &str) -> Self {
        if err.is_timeout() {
            return Self::RequestTimeout;
        }
        if err.is_connect() {
            return Self::RequestFailed(format!("Connection failed ({}): {}", context, err));
        }
        Self::RequestFailed(format!("{}: {}", context, err))
    }

    /// Convenience constructor for `CsrfTokenNotFound`.
    pub fn csrf_not_found() -> Self {
        Self::CsrfTokenNotFound
    }
}

impl From<reqwest::Error> for ApiError {
    /// Converts a `reqwest::Error` into an `ApiError` with a generic context.
    fn from(err: reqwest::Error) -> Self {
        Self::from_reqwest_error(err, "reqwest")
    }
}

impl From<reqwest::header::InvalidHeaderValue> for ApiError {
    /// Converts an invalid header value error into a `CacheError`.
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Self::CacheError(format!("Invalid header value: {}", err))
    }
}

impl From<serde_json::Error> for ApiError {
    /// Converts a JSON serialization/deserialization error into a `JsonError`.
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    /// Converts an I/O error into an `IoError`.
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<String> for ApiError {
    /// Converts a string message into the `Other` variant.
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

impl From<&str> for ApiError {
    /// Converts a string slice into the `Other` variant.
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}
