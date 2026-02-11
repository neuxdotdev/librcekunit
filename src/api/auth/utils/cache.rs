use crate::handler::error::ApiError;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheData {
    pub cookies: Vec<Cookie>,
    pub csrf_token: String,
    pub logged_in: bool,
    pub timestamp: i64,
}
impl CacheData {
    pub fn with_csrf_token(mut self, new_token: String) -> Self {
        self.csrf_token = new_token;
        self.timestamp = now();
        self
    }
    pub fn is_fresh(&self, max_age_seconds: i64) -> bool {
        now() - self.timestamp < max_age_seconds
    }
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
#[derive(Clone)]
pub struct CacheManager {
    cache_dir: PathBuf,
    cache_file: PathBuf,
}
impl CacheManager {
    pub fn new() -> Result<Self, ApiError> {
        let proj_dirs = ProjectDirs::from("com", "cekunit", "libcekunit")
            .ok_or_else(|| ApiError::CacheError("Cannot determine cache directory".to_string()))?;
        let cache_dir = proj_dirs.cache_dir().to_path_buf();
        let cache_file = cache_dir.join("session.json");
        fs::create_dir_all(&cache_dir)
            .map_err(|e| ApiError::CacheError(format!("Failed to create cache dir: {}", e)))?;
        Ok(Self {
            cache_dir,
            cache_file,
        })
    }
    pub fn with_paths(cache_dir: PathBuf, cache_file: PathBuf) -> Self {
        Self {
            cache_dir,
            cache_file,
        }
    }
    pub fn save(&self, data: &CacheData) -> Result<(), ApiError> {
        let json = serde_json::to_string_pretty(data)?;
        fs::write(&self.cache_file, json)
            .map_err(|e| ApiError::CacheError(format!("Failed to write cache: {}", e)))
    }
    pub fn load(&self) -> Result<Option<CacheData>, ApiError> {
        if !self.cache_file.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.cache_file)
            .map_err(|e| ApiError::CacheError(format!("Failed to read cache: {}", e)))?;
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
    pub fn update_csrf_token(&self, new_token: String) -> Result<(), ApiError> {
        if let Some(data) = self.load()? {
            let updated = data.with_csrf_token(new_token);
            self.save(&updated)?;
        }
        Ok(())
    }
    pub fn load_fresh(&self, max_age_seconds: i64) -> Result<Option<CacheData>, ApiError> {
        match self.load()? {
            Some(data) if data.is_fresh(max_age_seconds) => Ok(Some(data)),
            _ => Ok(None),
        }
    }
    pub fn cache_file_path(&self) -> &Path {
        &self.cache_file
    }
    pub fn cache_dir_path(&self) -> &Path {
        &self.cache_dir
    }
}
impl Default for CacheManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            let dir = PathBuf::from("./cache");
            let _ = fs::create_dir_all(&dir);
            Self {
                cache_dir: dir.clone(),
                cache_file: dir.join("session.json"),
            }
        })
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
