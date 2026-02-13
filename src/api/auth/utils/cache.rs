//! Session cache management for authentication data.
//!
//! This module provides persistent storage for session cookies and CSRF tokens
//! using the system's cache directory. It allows the application to maintain
//! login state across runs and provides utilities for loading, saving, and
//! validating cached sessions.

use crate::handler::error::ApiError;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a cached session, including cookies and CSRF token.
///
/// This structure is serialized to JSON and stored in the cache directory.
/// It contains all necessary data to resume an authenticated session without
/// re‑logging in.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheData {
    /// List of cookies associated with the session.
    pub cookies: Vec<Cookie>,
    /// The CSRF token extracted from the login page or a subsequent page.
    pub csrf_token: String,
    /// Flag indicating whether the session is considered logged in.
    ///
    /// This is set to `true` after a successful login.
    pub logged_in: bool,
    /// Unix timestamp (seconds) when this cache entry was last updated.
    pub timestamp: i64,
}

impl CacheData {
    /// Creates a new `CacheData` instance with an updated CSRF token.
    ///
    /// This method updates the CSRF token and resets the timestamp to the current time.
    /// It is typically used when a fresh token is needed, e.g., before a POST request.
    ///
    /// # Arguments
    /// * `new_token` - The new CSRF token to store.
    ///
    /// # Returns
    /// A new `CacheData` instance with the updated token and current timestamp.
    pub fn with_csrf_token(mut self, new_token: String) -> Self {
        self.csrf_token = new_token;
        self.timestamp = now();
        self
    }

    /// Checks whether the cached session is still fresh (not expired).
    ///
    /// # Arguments
    /// * `max_age_seconds` - Maximum allowed age of the cache in seconds.
    ///
    /// # Returns
    /// `true` if the cache was created or updated less than `max_age_seconds` ago.
    pub fn is_fresh(&self, max_age_seconds: i64) -> bool {
        now() - self.timestamp < max_age_seconds
    }
}

/// Represents a single HTTP cookie.
///
/// Cookies are stored with their attributes to allow accurate reconstruction
/// of the `Cookie` header for subsequent requests.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cookie {
    /// Name of the cookie.
    pub name: String,
    /// Value of the cookie.
    pub value: String,
    /// Domain for which the cookie is valid.
    pub domain: String,
    /// Path within the domain for which the cookie is valid.
    pub path: String,
    /// Whether the cookie is marked as `HttpOnly` (not accessible to JavaScript).
    pub http_only: bool,
    /// Whether the cookie is marked as `Secure` (only sent over HTTPS).
    pub secure: bool,
}

/// Manages reading and writing the session cache to the filesystem.
///
/// The cache is stored as a JSON file in a platform‑specific cache directory
/// (e.g., `~/.cache/cekunit/libcekunit/session.json` on Linux). The manager
/// provides methods to save, load, clear, and update the cache, as well as
/// to obtain paths to the cache file and directory.
#[derive(Clone)]
pub struct CacheManager {
    /// Directory where the cache file resides.
    cache_dir: PathBuf,
    /// Full path to the cache file (usually `cache_dir/session.json`).
    cache_file: PathBuf,
}

impl CacheManager {
    /// Creates a new `CacheManager` using the default system cache directory.
    ///
    /// The cache directory is determined via `directories::ProjectDirs` using the
    /// qualifier `"com"`, organization `"cekunit"`, and application `"libcekunit"`.
    /// If the directory cannot be created, an error is returned.
    ///
    /// # Errors
    /// Returns [`ApiError::CacheError`] if:
    /// - The system cache directory cannot be determined.
    /// - The cache directory cannot be created.
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

    /// Creates a `CacheManager` with custom paths.
    ///
    /// This is primarily useful for testing or when an alternative cache location
    /// is required.
    ///
    /// # Arguments
    /// * `cache_dir` - The directory to store the cache file.
    /// * `cache_file` - The full path to the cache file.
    pub fn with_paths(cache_dir: PathBuf, cache_file: PathBuf) -> Self {
        Self {
            cache_dir,
            cache_file,
        }
    }

    /// Saves the given cache data to the cache file.
    ///
    /// The data is serialized to JSON with pretty formatting.
    ///
    /// # Arguments
    /// * `data` - The cache data to save.
    ///
    /// # Errors
    /// Returns [`ApiError`] if serialization or file writing fails.
    pub fn save(&self, data: &CacheData) -> Result<(), ApiError> {
        let json = serde_json::to_string_pretty(data)?;
        fs::write(&self.cache_file, json)
            .map_err(|e| ApiError::CacheError(format!("Failed to write cache: {}", e)))
    }

    /// Loads the cache data from the cache file.
    ///
    /// If the file does not exist, returns `Ok(None)`. If the file exists but cannot
    /// be read or parsed, an error is returned.
    ///
    /// # Errors
    /// Returns [`ApiError`] if reading the file or parsing JSON fails.
    pub fn load(&self) -> Result<Option<CacheData>, ApiError> {
        if !self.cache_file.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.cache_file)
            .map_err(|e| ApiError::CacheError(format!("Failed to read cache: {}", e)))?;
        let data: CacheData = serde_json::from_str(&content)?;
        Ok(Some(data))
    }

    /// Deletes the cache file if it exists.
    ///
    /// # Errors
    /// Returns [`ApiError`] if the file exists but cannot be removed.
    pub fn clear(&self) -> Result<(), ApiError> {
        if self.cache_file.exists() {
            fs::remove_file(&self.cache_file)
                .map_err(|e| ApiError::CacheError(format!("Failed to clear cache: {}", e)))?;
        }
        Ok(())
    }

    /// Updates the CSRF token in the cache.
    ///
    /// If a cache entry exists, it is loaded, its CSRF token is replaced,
    /// and the timestamp is updated. If no cache exists, nothing is done.
    ///
    /// # Arguments
    /// * `new_token` - The new CSRF token.
    ///
    /// # Errors
    /// Returns [`ApiError`] if loading, updating, or saving fails.
    pub fn update_csrf_token(&self, new_token: String) -> Result<(), ApiError> {
        if let Some(data) = self.load()? {
            let updated = data.with_csrf_token(new_token);
            self.save(&updated)?;
        }
        Ok(())
    }

    /// Loads the cache only if it is fresh (not expired).
    ///
    /// # Arguments
    /// * `max_age_seconds` - Maximum allowed age of the cache in seconds.
    ///
    /// # Returns
    /// - `Ok(Some(data))` if the cache exists and is fresh.
    /// - `Ok(None)` if the cache does not exist or is stale.
    /// - `Err(ApiError)` if loading fails.
    pub fn load_fresh(&self, max_age_seconds: i64) -> Result<Option<CacheData>, ApiError> {
        match self.load()? {
            Some(data) if data.is_fresh(max_age_seconds) => Ok(Some(data)),
            _ => Ok(None),
        }
    }

    /// Returns a reference to the cache file path.
    pub fn cache_file_path(&self) -> &Path {
        &self.cache_file
    }

    /// Returns a reference to the cache directory path.
    pub fn cache_dir_path(&self) -> &Path {
        &self.cache_dir
    }
}

impl Default for CacheManager {
    /// Creates a default `CacheManager` using the system cache directory.
    ///
    /// If the system cache directory cannot be determined or created,
    /// it falls back to a local `./cache` directory. This fallback ensures
    /// that the application can still run even if the system cache is unavailable.
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

/// Returns the current Unix timestamp in seconds.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
