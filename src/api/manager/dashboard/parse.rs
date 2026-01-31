use super::structs::{DashboardError, NasabahData, PaginationInfo, UserInfo};
use select::document::Document;
use select::predicate::{Attr, Class, Name, Predicate};

pub fn parse_user_info(doc: &Document) -> Result<UserInfo, DashboardError> {
    if let Some(header) = doc.find(Class("header-info")).next() {
        let name = header
            .find(Name("h6"))
            .next()
            .map(|n| n.text().trim().to_string());
        let email = header
            .find(Name("p"))
            .next()
            .map(|p| p.text().trim().to_string());

        if let (Some(n), Some(e)) = (name, email)
            && !n.is_empty()
            && !e.is_empty()
        {
            return Ok(UserInfo { name: n, email: e });
        }
    }

    if let Some(h1) = doc.find(Name("h1")).next() {
        let text = h1.text();
        if text.contains("Hello,") {
            let name = text.replace("Hello,", "").trim().to_string();
            if !name.is_empty() {
                return Ok(UserInfo {
                    name,
                    email: String::new(),
                });
            }
        }
    }

    if let Some(dropdown) = doc.find(Attr("class", "dropdown-menu-end")).next() {
        let name = dropdown
            .find(Name("h6"))
            .next()
            .map(|n| n.text().trim().to_string());
        let email = dropdown
            .find(Name("span"))
            .next()
            .map(|s| s.text().trim().to_string());

        if let (Some(n), Some(e)) = (name, email)
            && !n.is_empty()
        {
            return Ok(UserInfo { name: n, email: e });
        }
    }

    Err(DashboardError::Parse(
        "User information not found in HTML".into(),
    ))
}

pub fn parse_table_data(doc: &Document) -> Result<Vec<NasabahData>, DashboardError> {
    let table = doc
        .find(Attr("id", "cekunit-table"))
        .next()
        .ok_or_else(|| DashboardError::Parse("Table with id 'cekunit-table' not found".into()))?;

    let tbody = table
        .find(Name("tbody"))
        .next()
        .ok_or_else(|| DashboardError::Parse("Table tbody not found".into()))?;

    let mut result = Vec::new();
    for (row_idx, row) in tbody.find(Name("tr")).enumerate() {
        let cells: Vec<String> = row
            .find(Name("td"))
            .map(|c| c.text().trim().to_string())
            .collect();

        if cells.len() >= 17 {
            let nasabah = NasabahData {
                no: (row_idx + 1) as u32,
                no_perjanjian: cells.get(1).cloned().unwrap_or_default(),
                nama_nasabah: cells.get(2).cloned().unwrap_or_default(),
                nopol: cells.get(3).cloned().unwrap_or_default(),
                coll: cells.get(4).cloned().unwrap_or_default(),
                pic: cells.get(5).cloned().unwrap_or_default(),
                kategori: cells.get(6).cloned().unwrap_or_default(),
                jto: cells.get(7).cloned().unwrap_or_default(),
                no_rangka: cells.get(8).cloned().unwrap_or_default(),
                no_mesin: cells.get(9).cloned().unwrap_or_default(),
                merk: cells.get(10).cloned().unwrap_or_default(),
                type_unit: cells.get(11).cloned().unwrap_or_default(),
                warna: cells.get(12).cloned().unwrap_or_default(),
                status: cells.get(13).cloned().unwrap_or_default(),
                actual_penyelesaian: cells.get(14).cloned().unwrap_or_default(),
                angsuran_ke: cells.get(15).cloned().unwrap_or_default(),
                tenor: cells.get(16).cloned().unwrap_or_default(),
            };
            result.push(nasabah);
        }
    }

    Ok(result)
}

pub fn parse_pagination_info(doc: &Document) -> Result<PaginationInfo, DashboardError> {
    let mut current_page = 1;
    let mut total_pages = 1;
    let mut total_data = 0;
    let per_page = 20;

    if let Some(pagination_div) = doc.find(Class("text-center")).nth(1) {
        let text = pagination_div.text();
        let parts: Vec<&str> = text.split_whitespace().collect();

        if parts.len() >= 6 {
            total_data = parts[5].parse().unwrap_or(0);
        }
    }

    if let Some(active_page) = doc.find(Class("page-item").and(Class("active"))).next()
        && let Some(page_span) = active_page.find(Name("span")).next()
    {
        current_page = page_span.text().trim().parse().unwrap_or(1);
    }

    if total_data > 0 && per_page > 0 {
        total_pages = (total_data as f64 / per_page as f64).ceil() as u32;
    }

    Ok(PaginationInfo {
        current_page,
        total_pages,
        total_data,
        per_page,
    })
}

pub fn parse_dashboard_html(
    html: &str,
) -> Result<(UserInfo, Vec<NasabahData>, PaginationInfo), DashboardError> {
    let document = Document::from(html);
    let user_info = parse_user_info(&document)?;
    let data = parse_table_data(&document)?;
    let pagination = parse_pagination_info(&document)?;
    Ok((user_info, data, pagination))
}
