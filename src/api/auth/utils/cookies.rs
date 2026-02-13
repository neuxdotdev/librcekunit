//! Cookie handling utilities for HTTP requests and responses.
//!
//! This module provides functions to extract cookies from `Set-Cookie` headers,
//! build a `Cookie` header value from a collection of cookies, and add cookies
//! to a request's header map. It also includes a parser for individual `Set-Cookie`
//! strings.

use crate::handler::error::ApiError;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, SET_COOKIE};
use std::collections::HashMap;

/// Extracts all cookies from the `Set-Cookie` headers of an HTTP response.
///
/// This function iterates over all `Set-Cookie` header values, parses each one
/// using [`parse_set_cookie`], and inserts the resulting name‑value pairs into
/// a `HashMap`. If multiple cookies with the same name are received, later ones
/// will overwrite earlier ones (which is generally the intended behaviour).
///
/// # Arguments
/// * `headers` - A reference to the response [`HeaderMap`].
///
/// # Returns
/// A `HashMap<String, String>` where keys are cookie names and values are
/// the corresponding cookie values.
///
/// # Example
/// ```
/// use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
/// use cekunit_client::api::auth::utils::cookies::extract_cookies;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(SET_COOKIE, HeaderValue::from_static("session=abc123; Path=/"));
/// headers.append(SET_COOKIE, HeaderValue::from_static("theme=dark"));
///
/// let cookies = extract_cookies(&headers);
/// assert_eq!(cookies.get("session"), Some(&"abc123".to_string()));
/// assert_eq!(cookies.get("theme"), Some(&"dark".to_string()));
/// ```
pub fn extract_cookies(headers: &HeaderMap) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    for value in headers.get_all(SET_COOKIE) {
        if let Ok(cookie_str) = value.to_str() {
            if let Some((name, value)) = parse_set_cookie(cookie_str) {
                cookies.insert(name, value);
            }
        }
    }
    cookies
}

/// Builds a `Cookie` header value string from a collection of cookies.
///
/// The resulting string is formatted as `name1=value1; name2=value2; ...`,
/// suitable for use in the `Cookie` header of an HTTP request.
///
/// # Arguments
/// * `cookies` - A map of cookie names to values.
///
/// # Returns
/// A string containing all cookies joined by `; `.
///
/// # Example
/// ```
/// use std::collections::HashMap;
/// use cekunit_client::api::auth::utils::cookies::build_cookie_header;
///
/// let mut cookies = HashMap::new();
/// cookies.insert("session".to_string(), "xyz".to_string());
/// cookies.insert("theme".to_string(), "dark".to_string());
///
/// let header = build_cookie_header(&cookies);
/// // The order may vary, so we check both possibilities.
/// assert!(header == "session=xyz; theme=dark" || header == "theme=dark; session=xyz");
/// ```
pub fn build_cookie_header(cookies: &HashMap<String, String>) -> String {
    cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Adds cookies to the `Cookie` header of a request's [`HeaderMap`].
///
/// If the cookie map is not empty, this function builds a cookie header string
/// using [`build_cookie_header`] and inserts it into the provided `HeaderMap`
/// under the `COOKIE` key. Any existing `Cookie` header is overwritten.
///
/// # Arguments
/// * `headers` - Mutable reference to the request's [`HeaderMap`].
/// * `cookies` - A map of cookie names to values.
///
/// # Returns
/// * `Ok(())` on success.
/// * `Err(ApiError::CacheError)` if the cookie header string contains invalid
///   characters (very unlikely for typical cookie values).
///
/// # Example
/// ```
/// use std::collections::HashMap;
/// use reqwest::header::{HeaderMap, COOKIE};
/// use cekunit_client::api::auth::utils::cookies::add_cookies_to_headers;
///
/// let mut headers = HeaderMap::new();
/// let mut cookies = HashMap::new();
/// cookies.insert("token".to_string(), "secret".to_string());
///
/// add_cookies_to_headers(&mut headers, &cookies).unwrap();
/// assert_eq!(headers.get(COOKIE).unwrap(), "token=secret");
/// ```
pub fn add_cookies_to_headers(
    headers: &mut HeaderMap,
    cookies: &HashMap<String, String>,
) -> Result<(), ApiError> {
    if !cookies.is_empty() {
        let cookie_str = build_cookie_header(cookies);
        let header_value = HeaderValue::from_str(&cookie_str)
            .map_err(|e| ApiError::CacheError(format!("Invalid cookie header: {}", e)))?;
        headers.insert(COOKIE, header_value);
    }
    Ok(())
}

/// Parses a single `Set-Cookie` header string into a cookie name and value.
///
/// This function extracts the first name‑value pair from a `Set-Cookie` string,
/// ignoring any additional attributes (like `Path`, `Domain`, `HttpOnly`, etc.).
/// It returns `None` if the string does not contain a valid `name=value` pair.
///
/// # Arguments
/// * `cookie_str` - A raw `Set-Cookie` header value.
///
/// # Returns
/// * `Some((name, value))` where `name` and `value` are the parsed strings.
/// * `None` if the string is malformed (e.g., no `=`, empty name).
///
/// # Examples
/// ```
/// use cekunit_client::api::auth::utils::cookies::parse_set_cookie;
///
/// let (name, value) = parse_set_cookie("session=abc123; Path=/; HttpOnly").unwrap();
/// assert_eq!(name, "session");
/// assert_eq!(value, "abc123");
///
/// assert!(parse_set_cookie("=novalue").is_none());
/// ```
fn parse_set_cookie(cookie_str: &str) -> Option<(String, String)> {
    let mut parts = cookie_str.splitn(2, '=');
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    let rest = parts.next()?;
    let value = rest.split(';').next().unwrap_or(rest).trim();
    Some((name.to_string(), value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};

    /// Tests that valid `Set-Cookie` strings are correctly parsed.
    #[test]
    fn test_parse_set_cookie_valid() {
        assert_eq!(
            parse_set_cookie("session=abc123; Path=/; HttpOnly"),
            Some(("session".to_string(), "abc123".to_string()))
        );
        assert_eq!(
            parse_set_cookie("name=value with space"),
            Some(("name".to_string(), "value with space".to_string()))
        );
        assert_eq!(
            parse_set_cookie("empty="),
            Some(("empty".to_string(), "".to_string()))
        );
    }

    /// Tests that malformed `Set-Cookie` strings return `None`.
    #[test]
    fn test_parse_set_cookie_invalid() {
        assert_eq!(parse_set_cookie("=novalue"), None);
        assert_eq!(parse_set_cookie("justname"), None);
        assert_eq!(parse_set_cookie(""), None);
    }

    /// Tests extraction of multiple cookies from response headers.
    #[test]
    fn test_extract_cookies_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("set-cookie"),
            HeaderValue::from_static("user=alice; Path=/"),
        );
        headers.append(
            HeaderName::from_static("set-cookie"),
            HeaderValue::from_static("lang=en; HttpOnly"),
        );
        headers.append(
            HeaderName::from_static("set-cookie"),
            HeaderValue::from_static("theme=dark"),
        );

        let cookies = extract_cookies(&headers);
        assert_eq!(cookies.len(), 3);
        assert_eq!(cookies.get("user"), Some(&"alice".to_string()));
        assert_eq!(cookies.get("lang"), Some(&"en".to_string()));
        assert_eq!(cookies.get("theme"), Some(&"dark".to_string()));
    }

    /// Tests building a `Cookie` header string from a cookie map.
    #[test]
    fn test_build_cookie_header() {
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "xyz".to_string());
        cookies.insert("theme".to_string(), "dark".to_string());

        let header = build_cookie_header(&cookies);
        // Order is not guaranteed, so accept both possibilities.
        assert!(header == "session=xyz; theme=dark" || header == "theme=dark; session=xyz");
    }

    /// Tests adding cookies to a `HeaderMap`.
    #[test]
    fn test_add_cookies_to_headers() {
        let mut cookies = HashMap::new();
        cookies.insert("token".to_string(), "secret".to_string());

        let mut headers = HeaderMap::new();
        add_cookies_to_headers(&mut headers, &cookies).unwrap();

        assert_eq!(headers.get(COOKIE).unwrap(), "token=secret");
    }

    /// Tests that adding an empty cookie map does not insert a `Cookie` header.
    #[test]
    fn test_add_cookies_to_headers_empty() {
        let cookies = HashMap::new();
        let mut headers = HeaderMap::new();
        add_cookies_to_headers(&mut headers, &cookies).unwrap();
        assert!(headers.get(COOKIE).is_none());
    }
}
