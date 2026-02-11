use crate::handler::error::ApiError;
use select::document::Document;
use select::predicate::Attr;
const CSRF_INPUT_SELECTOR: Attr<&str, &str> = Attr("name", "_token");
const CSRF_META_SELECTOR: Attr<&str, &str> = Attr("name", "csrf-token");
pub fn extract_csrf_token(html: &str) -> Result<String, ApiError> {
    if let Some(token) = extract_from_input(html) {
        return Ok(token);
    }
    if let Some(token) = extract_from_meta(html) {
        return Ok(token);
    }
    Err(ApiError::CsrfTokenNotFound)
}
fn extract_from_input(html: &str) -> Option<String> {
    let doc = Document::from(html);
    doc.find(CSRF_INPUT_SELECTOR)
        .next()
        .and_then(|node| node.attr("value"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
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
    #[test]
    fn test_extract_from_input_success() {
        let html = r#"<input type="hidden" name="_token" value="abc123xyz">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "abc123xyz");
    }
    #[test]
    fn test_extract_from_input_with_whitespace() {
        let html = r#"<input name="_token" value="   spaced-token   ">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "spaced-token");
    }
    #[test]
    fn test_extract_from_input_empty_value() {
        let html = r#"<input name="_token" value="">"#;
        assert!(extract_csrf_token(html).is_err());
    }
    #[test]
    fn test_extract_from_meta_success() {
        let html = r#"<meta name="csrf-token" content="meta-token-456">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "meta-token-456");
    }
    #[test]
    fn test_extract_from_meta_with_whitespace() {
        let html = r#"<meta name="csrf-token" content="   token-with-spaces   ">"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "token-with-spaces");
    }
    #[test]
    fn test_extract_from_meta_empty_content() {
        let html = r#"<meta name="csrf-token" content="">"#;
        assert!(extract_csrf_token(html).is_err());
    }
    #[test]
    fn test_no_token() {
        let html = r#"<html><body>No token here</body></html>"#;
        assert!(extract_csrf_token(html).is_err());
    }
    #[test]
    fn test_both_present_input_takes_precedence() {
        let html = r#"
            <input name="_token" value="input-token">
            <meta name="csrf-token" content="meta-token">
        "#;
        assert_eq!(extract_csrf_token(html).unwrap(), "input-token");
    }
    #[test]
    fn test_malformed_html_graceful() {
        let html = r#"<input name="_token" value="abc"</input>"#;
        assert_eq!(extract_csrf_token(html).unwrap(), "abc");
    }
}
