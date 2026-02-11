use comfy_table::{ContentArrangement, Table, presets};
use librcekunit::{ApiError, CekUnitClient};
use scraper::{Html, Selector};
use std::fs;
#[test]
fn test_dashboard_workflow() -> Result<(), ApiError> {
    let _ = env_logger::builder().is_test(true).try_init();
    println!("\n=== TEST DASHBOARD WORKFLOW ===\n");
    let mut client = CekUnitClient::new()?;
    let cache = client.cache_file_path();
    if cache.exists() {
        fs::remove_file(&cache).map_err(|e| ApiError::CacheError(e.to_string()))?;
    }
    client.login()?;
    assert!(cache.exists());
    let dashboard = client.dashboard()?;
    let html = dashboard.get_dashboard(Some(1), None, None, None)?;
    assert!(!html.is_empty());
    let doc = Html::parse_document(&html);
    let table_sel = Selector::parse("table").unwrap();
    let th_sel = Selector::parse("thead th").unwrap();
    let tr_sel = Selector::parse("tbody tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let table_node = doc.select(&table_sel).next().expect("table not found");
    let headers: Vec<String> = table_node
        .select(&th_sel)
        .map(|e| e.text().collect::<String>())
        .collect();
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(headers);
    for row in table_node.select(&tr_sel).take(5) {
        let cells: Vec<String> = row
            .select(&td_sel)
            .map(|td| td.text().collect::<String>().trim().to_string())
            .collect();
        if !cells.is_empty() {
            table.add_row(cells);
        }
    }
    println!("{}", table);
    let token = dashboard.get_csrf_token()?;
    assert!(!token.is_empty());
    let csv = dashboard.export_cekunit("csv", "no", "asc")?;
    assert!(!csv.is_empty());
    fs::write("export_test.csv", &csv).map_err(|e| ApiError::CacheError(e.to_string()))?;
    client.logout()?;
    assert!(!cache.exists());
    assert!(dashboard.get_dashboard(Some(1), None, None, None).is_err());
    println!("\n=== DONE ===\n");
    Ok(())
}
