use crate::handler::error::ApiError;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheData {
    pub cookies: Vec<Cookie>,
    pub csrf_token: String,
    pub logged_in: bool,
    pub timestamp: i64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub http_only: bool,
    pub secure: bool,
}
pub struct CacheManager {
    pub cache_dir: PathBuf,
    pub cache_file: PathBuf,
}
impl CacheManager {
    pub fn new() -> Result<Self, ApiError> {
        let proj_dirs = ProjectDirs::from("com", "cekunit", "libcekunit")
            .ok_or_else(|| ApiError::CacheError("Cannot determine cache directory".to_string()))?;
        let cache_dir = proj_dirs.cache_dir().to_path_buf();
        let cache_file = cache_dir.join("session.json");
        fs::create_dir_all(&cache_dir).map_err(|e| {
            ApiError::CacheError(format!("Failed to create cache directory: {}", e))
        })?;
        Ok(Self {
            cache_dir,
            cache_file,
        })
    }
    pub fn save(&self, data: &CacheData) -> Result<(), ApiError> {
        let json = serde_json::to_string_pretty(data)?;
        fs::write(&self.cache_file, json)
            .map_err(|e| ApiError::CacheError(format!("Failed to write cache file: {}", e)))
    }
    pub fn load(&self) -> Result<Option<CacheData>, ApiError> {
        if !self.cache_file.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.cache_file)
            .map_err(|e| ApiError::CacheError(format!("Failed to read cache file: {}", e)))?;
        let data: CacheData = serde_json::from_str(&content)?;
        Ok(Some(data))
    }
    pub fn clear(&self) -> Result<(), ApiError> {
        if self.cache_file.exists() {
            fs::remove_file(&self.cache_file)
                .map_err(|e| ApiError::CacheError(format!("Failed to clear cache: {}", e)))?;
        }
        Ok(())
    }
    pub fn get_cache_file_path(&self) -> PathBuf {
        self.cache_file.clone()
    }
    pub fn with_custom_path(cache_dir: PathBuf, cache_file: PathBuf) -> Self {
        Self {
            cache_dir,
            cache_file,
        }
    }
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
    pub fn cache_file(&self) -> &Path {
        &self.cache_file
    }
}
