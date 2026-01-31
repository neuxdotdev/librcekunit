use super::parse::parse_dashboard_html;
use super::structs::{DashboardData, DashboardError, SearchParams};
use crate::api::auth::utils::cache::CacheData;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;

pub struct DashboardClient {
    client: Client,
    base_url: String,
    cache_data: Option<CacheData>,
    timeout: Duration,
}

impl DashboardClient {
    pub fn new(base_url: String, cache_data: Option<CacheData>) -> Result<Self, DashboardError> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0")
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| DashboardError::Request(format!("Client build failed: {}", e)))?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            cache_data,
            timeout: Duration::from_secs(30),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn build_headers(&self) -> Result<HeaderMap, DashboardError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0"
                .parse()
                .unwrap(),
        );

        // Tambahkan cookies dari cache jika ada
        if let Some(cache) = &self.cache_data {
            let cookie_map: HashMap<String, String> = cache
                .cookies
                .iter()
                .map(|c| (c.name.clone(), c.value.clone()))
                .collect();

            if !cookie_map.is_empty() {
                let cookie_str = cookie_map
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<String>>()
                    .join("; ");

                headers.insert(
                    "Cookie",
                    cookie_str
                        .parse()
                        .map_err(|e| DashboardError::Cache(format!("Invalid cookie: {}", e)))?,
                );
            }
        }

        Ok(headers)
    }

    fn fetch_html(&self, url: &str) -> Result<String, DashboardError> {
        let headers = self.build_headers()?;

        let response = self
            .client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .map_err(|e| DashboardError::Request(format!("GET request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            return Err(DashboardError::Request(format!(
                "HTTP {} for {}",
                status, url
            )));
        }

        response
            .text()
            .map_err(|e| DashboardError::Request(format!("Failed reading body: {}", e)))
    }

    pub fn fetch_dashboard(&self, page: Option<u32>) -> Result<DashboardData, DashboardError> {
        let url = if let Some(page) = page {
            format!("{}/dashboard?page={}", self.base_url, page)
        } else {
            format!("{}/dashboard", self.base_url)
        };

        log::debug!("Fetching dashboard from: {}", url);
        let html = self.fetch_html(&url)?;

        let (user_info, data, pagination) = parse_dashboard_html(&html)?;

        Ok(DashboardData {
            data,
            pagination,
            user_info,
        })
    }

    pub fn search_data(&self, params: &SearchParams) -> Result<DashboardData, DashboardError> {
        let mut url = format!("{}/dashboard", self.base_url);
        let mut query_params = Vec::new();

        if !params.query.is_empty() {
            query_params.push(format!("search={}", urlencoding::encode(&params.query)));
        }

        if let Some(column) = &params.column {
            query_params.push(format!("sort={}", urlencoding::encode(column)));
        }

        if let Some(sort_col) = &params.sort_column {
            query_params.push(format!("sortColumn={}", urlencoding::encode(sort_col)));
        }

        if let Some(sort_dir) = &params.sort_direction {
            query_params.push(format!("sortDirection={}", urlencoding::encode(sort_dir)));
        }

        if let Some(page) = params.page {
            query_params.push(format!("page={}", page));
        }

        if !query_params.is_empty() {
            url.push_str(&format!("?{}", query_params.join("&")));
        }

        log::debug!("Searching data from: {}", url);
        let html = self.fetch_html(&url)?;

        let (user_info, data, pagination) = parse_dashboard_html(&html)?;

        Ok(DashboardData {
            data,
            pagination,
            user_info,
        })
    }

    pub fn export_data(&self, format: &str) -> Result<Vec<u8>, DashboardError> {
        let url = format!(
            "{}/dashboard/cekunit/export?format={}",
            self.base_url, format
        );

        log::debug!("Exporting data from: {}", url);
        let headers = self.build_headers()?;

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .map_err(|e| DashboardError::Request(format!("Export request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DashboardError::Request(format!(
                "Export failed with status: {}",
                response.status()
            )));
        }

        response
            .bytes()
            .map_err(|e| DashboardError::Request(format!("Failed reading response bytes: {}", e)))
            .map(|bytes| bytes.to_vec())
    }

    pub fn get_unique_values(&self, column: &str) -> Result<Vec<String>, DashboardError> {
        let url = format!(
            "{}/dashboard/cekunit/get-unique-values?column={}",
            self.base_url, column
        );

        log::debug!("Getting unique values from: {}", url);
        let headers = self.build_headers()?;

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .map_err(|e| DashboardError::Request(format!("GET request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DashboardError::Request(format!(
                "Failed to get unique values: {}",
                response.status()
            )));
        }

        response
            .json::<Vec<String>>()
            .map_err(|e| DashboardError::Json(format!("JSON parse failed: {}", e)))
    }

    pub fn delete_by_category(&self, column: &str, value: &str) -> Result<bool, DashboardError> {
        let url = format!("{}/dashboard/cekunit/delete-by-category", self.base_url);

        let mut headers = self.build_headers()?;
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let mut params = HashMap::new();
        params.insert("column", column);
        params.insert("value", value);

        // Cari token CSRF dari cache
        if let Some(cache) = &self.cache_data {
            params.insert("_token", &cache.csrf_token);
        }

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&params)
            .timeout(self.timeout)
            .send()
            .map_err(|e| DashboardError::Request(format!("DELETE request failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    pub fn delete_all(&self) -> Result<bool, DashboardError> {
        let url = format!("{}/dashboard/delete-all", self.base_url);

        let mut headers = self.build_headers()?;
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let mut params: HashMap<&str, String> = HashMap::new();

        // Cari token CSRF dari cache
        if let Some(cache) = &self.cache_data {
            params.insert("_token", cache.csrf_token.clone());
        }
        params.insert("_method", "DELETE".to_string());

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&params)
            .timeout(self.timeout)
            .send()
            .map_err(|e| DashboardError::Request(format!("DELETE ALL request failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    pub fn update_record(
        &self,
        id: u32,
        data: HashMap<&str, &str>,
    ) -> Result<bool, DashboardError> {
        let url = format!("{}/cekunit/{}", self.base_url, id);

        let mut headers = self.build_headers()?;
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let mut params = HashMap::new();
        // Tambahkan semua data
        for (key, value) in data {
            params.insert(key, value);
        }

        // Cari token CSRF dari cache
        if let Some(cache) = &self.cache_data {
            params.insert("_token", &cache.csrf_token);
        }
        params.insert("_method", "PUT");

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&params)
            .timeout(self.timeout)
            .send()
            .map_err(|e| DashboardError::Request(format!("UPDATE request failed: {}", e)))?;

        Ok(response.status().is_success())
    }
}
