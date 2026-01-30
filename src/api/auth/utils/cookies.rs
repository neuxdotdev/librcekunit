use crate::handler::error::ApiError;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, SET_COOKIE};
use std::collections::HashMap;
pub fn extract_cookies(headers: &HeaderMap) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    for value in headers.get_all(SET_COOKIE) {
        if let Ok(cookie_str) = value.to_str()
            && let Some((name, value)) = parse_cookie(cookie_str)
        {
            cookies.insert(name, value);
        }
    }
    cookies
}
pub fn parse_cookie(cookie_str: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = cookie_str.split(';').collect();
    if let Some(first_part) = parts.first() {
        let kv: Vec<&str> = first_part.splitn(2, '=').collect();
        if kv.len() == 2 {
            return Some((kv[0].trim().to_string(), kv[1].trim().to_string()));
        }
    }
    None
}
pub fn build_cookie_header(cookies: &HashMap<String, String>) -> String {
    cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>()
        .join("; ")
}
pub fn add_cookies_to_headers(
    headers: &mut HeaderMap,
    cookies: &HashMap<String, String>,
) -> Result<(), ApiError> {
    if !cookies.is_empty() {
        let cookie_str = build_cookie_header(cookies);
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&cookie_str)
                .map_err(|e| ApiError::CacheError(format!("Invalid cookie: {}", e)))?,
        );
    }
    Ok(())
}
