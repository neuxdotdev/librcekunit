use crate::handler::error::ApiError;
use select::document::Document;
use select::predicate::Attr;
pub fn extract_csrf_token(html: &str) -> Result<String, ApiError> {
    let document = Document::from(html);
    if let Some(token_input) = document.find(Attr("name", "_token")).next()
        && let Some(token_value) = token_input.attr("value")
    {
        return Ok(token_value.to_string());
    }
    if let Some(meta_tag) = document.find(Attr("name", "csrf-token")).next()
        && let Some(token_value) = meta_tag.attr("content")
    {
        return Ok(token_value.to_string());
    }
    Err(ApiError::CsrfTokenNotFound)
}
