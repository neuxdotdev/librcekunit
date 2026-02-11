use librcekunit::{ApiError, CekUnitClient};
use std::fs;
#[test]
fn test_input_user_workflow_minimal() -> Result<(), ApiError> {
    let _ = env_logger::builder().is_test(true).try_init();
    let mut client = CekUnitClient::new()?;
    let cache = client.cache_file_path();
    if cache.exists() {
        fs::remove_file(&cache).map_err(|e| ApiError::CacheError(e.to_string()))?;
    }
    client.login()?;
    assert!(cache.exists());
    let input = client.input_user()?;
    let html = input.get_input_user(Some(1), None, None, None, None, None)?;
    assert!(!html.is_empty());
    let csv = input.export_input_user("csv", "id", "asc", None, None, None)?;
    assert!(!csv.is_empty());
    fs::write("test_export.csv", &csv).map_err(|e| ApiError::CacheError(e.to_string()))?;
    let filtered = input.get_input_user(
        Some(1),
        Some("BP1274OR"),
        Some("nopol"),
        Some("asc"),
        None,
        None,
    )?;
    assert!(!filtered.is_empty());
    let csv_date = input.export_input_user(
        "csv",
        "created_at",
        "desc",
        None,
        Some("2026-02-01"),
        Some("2026-02-02"),
    )?;
    assert!(!csv_date.is_empty());
    client.logout()?;
    assert!(!cache.exists());
    Ok(())
}
