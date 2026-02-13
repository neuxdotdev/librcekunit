//! CSRF token extraction from HTML pages.
//!
//! This module provides functionality to extract Cross-Site Request Forgery (CSRF)
//! tokens from HTML responses. It supports two common locations:
//! - `<input type="hidden" name="_token" value="...">`
//! - `<meta name="csrf-token" content="...">`
//!
//! The extraction is performed using the `select` crate for HTML parsing.
//! The primary entry point is [`extract_csrf_token`], which returns the first
//! non‑empty token found, or an error if none exists.

use crate::handler::error::ApiError;
use select::document::Document;
use select::predicate::Attr;

/// Selector for an `<input>` element with `name="_token"`.
///
/// This matches hidden input fields commonly used by Laravel and similar frameworks
/// to store CSRF tokens.
const CSRF_INPUT_SELECTOR: Attr<&str, &str> = Attr("name", "_token");

/// Selector for a `<meta>` element with `name="csrf-token"`.
///
/// Some applications also expose the CSRF token in a meta tag for JavaScript access.
const CSRF_META_SELECTOR: Attr<&str, &str> = Attr("name", "csrf-token");

/// Extracts a CSRF token from an HTML string.
///
/// The function looks for the token in two places, in order:
/// 1. An `<input name="_token" value="...">` element.
/// 2. A `<meta name="csrf-token" content="...">` element.
///
/// The first non‑empty value found is returned. If both are missing or empty,
/// an error is returned.
///
/// # Arguments
/// * `html` - The HTML content as a string slice.
///
/// # Returns
/// * `Ok(String)` containing the token value (trimmed).
/// * `Err(ApiError::CsrfTokenNotFound)` if no token could be found.
///
/// # Examples
/// ```
/// use cekunit_client::api::auth::utils::token::extract_csrf_token;
///
/// let html = r#"<input type="hidden" name="_token" value="abc123">"#;
/// assert_eq!(extract_csrf_token(html).unwrap(), "abc123");
///
/// let html = r#"<meta name="csrf-token" content="xyz789">"#;
/// assert_eq!(extract_csrf_token(html).unwrap(), "xyz789");
///
/// let html = r#"<html><body>No token here</body></html>"#;
/// assert!(extract_csrf_token(html).is_err());
/// ```
pub fn extract_csrf_token(html: &str) -> Result<String, ApiError> {
    if let Some(token) = extract_from_input(html) {
        return Ok(token);
    }
    if let Some(token) = extract_from_meta(html) {
        return Ok(token);
    }
    Err(ApiError::CsrfTokenNotFound)
}

/// Attempts to extract a CSRF token from an `<input name="_token">` element.
///
/// Parses the HTML, finds the first matching input element, and returns its
/// `value` attribute after trimming. Returns `None` if the element is missing
/// or the value is empty.
///
/// # Arguments
/// * `html` - The HTML content as a string slice.
///
/// # Returns
/// * `Some(String)` if a token is found and non‑empty.
/// * `None` otherwise.
fn extract_from_input(html: &str) -> Option<String> {
    let doc = Document::from(html);
    doc.find(CSRF_INPUT_SELECTOR)
        .next()
        .and_then(|node| node.attr("value"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Attempts to extract a CSRF token from a `<meta name="csrf-token">` element.
///
/// Parses the HTML, finds the first matching meta tag, and returns its
/// `content` attribute after trimming. Returns `None` if the element is missing
/// or the content is empty.
///
/// # Arguments
/// * `html` - The HTML content as a string slice.
///
/// # Returns
/// * `Some(String)` if a token is found and non‑empty.
/// * `None` otherwise.
fn extract_from_meta(html: &str) -> Option<String> {
    let doc = Document::from(html);
    doc.find(CSRF_META_SELECTOR)
        .next()
        .and_then(|node| node.attr("content"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that a token is correctly extracted from an input element.
    #[test]
    fn test_extract_from_input_success() {
        let html = r#"<input type="hidden" name="_token" value="abc123xyz">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "abc123xyz");
    }

    /// Tests that whitespace around the token value is trimmed.
    #[test]
    fn test_extract_from_input_with_whitespace() {
        let html = r#"<input name="_token" value="   spaced-token   ">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "spaced-token");
    }

    /// Tests that an empty value in the input element results in an error.
    #[test]
    fn test_extract_from_input_empty_value() {
        let html = r#"<input name="_token" value="">"#;
        assert!(extract_csrf_token(html).is_err());
    }

    /// Tests that a token is correctly extracted from a meta tag.
    #[test]
    fn test_extract_from_meta_success() {
        let html = r#"<meta name="csrf-token" content="meta-token-456">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "meta-token-456");
    }

    /// Tests that whitespace around the meta content is trimmed.
    #[test]
    fn test_extract_from_meta_with_whitespace() {
        let html = r#"<meta name="csrf-token" content="   token-with-spaces   ">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "token-with-spaces");
    }

    /// Tests that an empty content in the meta tag results in an error.
    #[test]
    fn test_extract_from_meta_empty_content() {
        let html = r#"<meta name="csrf-token" content="">"#;
        assert!(extract_csrf_token(html).is_err());
    }

    /// Tests that when no token element exists, an error is returned.
    #[test]
    fn test_no_token() {
        let html = r#"<html><body>No token here</body></html>"#;
        assert!(extract_csrf_token(html).is_err());
    }

    /// Tests that when both input and meta are present, the input takes precedence.
    #[test]
    fn test_both_present_input_takes_precedence() {
        let html = r#"
            <input name="_token" value="input-token">
            <meta name="csrf-token" content="meta-token">
        "#;
        assert_eq!(extract_csrf_token(html).unwrap(), "input-token");
    }

    /// Tests that the parser is resilient to slightly malformed HTML.
    #[test]
    fn test_malformed_html_graceful() {
        let html = r#"<input name="_token" value="abc"</input>"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "abc");
    }
}
