use crate::handler::error::ApiError;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, SET_COOKIE};
use std::collections::HashMap;
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
pub fn build_cookie_header(cookies: &HashMap<String, String>) -> String {
    cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ")
}
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
    #[test]
    fn test_parse_set_cookie_invalid() {
        assert_eq!(parse_set_cookie("=novalue"), None);
        assert_eq!(parse_set_cookie("justname"), None);
        assert_eq!(parse_set_cookie(""), None);
    }
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
    #[test]
    fn test_build_cookie_header() {
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "xyz".to_string());
        cookies.insert("theme".to_string(), "dark".to_string());
        let header = build_cookie_header(&cookies);
        assert!(header == "session=xyz; theme=dark" || header == "theme=dark; session=xyz");
    }
    #[test]
    fn test_add_cookies_to_headers() {
        let mut cookies = HashMap::new();
        cookies.insert("token".to_string(), "secret".to_string());
        let mut headers = HeaderMap::new();
        add_cookies_to_headers(&mut headers, &cookies).unwrap();
        assert_eq!(headers.get(COOKIE).unwrap(), "token=secret");
    }
    #[test]
    fn test_add_cookies_to_headers_empty() {
        let cookies = HashMap::new();
        let mut headers = HeaderMap::new();
        add_cookies_to_headers(&mut headers, &cookies).unwrap();
        assert!(headers.get(COOKIE).is_none());
    }
}
