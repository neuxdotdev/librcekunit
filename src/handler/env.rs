use std::env;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum EnvError {
    #[error(" Environment variable '{0}' tidak ditemukan")]
    NotFound(String),
    #[error(" '{0}' tidak boleh kosong")]
    Empty(String),
    #[error(" '{0}' tidak valid: {1}")]
    Invalid(String, String),
    #[error(" Format URL tidak valid untuk '{0}': {1}")]
    InvalidUrl(String, String),
    #[error(" Endpoint '{0}' tidak boleh mengandung karakter ilegal: {1}")]
    InvalidEndpoint(String, String),
}
#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub user_email: String,
    pub user_password: String,
    pub base_url: String,
    pub login_endpoint: String,
    pub logout_endpoint: String,
    pub dashboard_endpoint: String,
    pub cekunit_export_endpoint: String,
    pub cekunit_unique_endpoint: String,
    pub cekunit_delete_category_endpoint: String,
    pub delete_all_endpoint: String,
    pub cekunit_item_endpoint: String,
    pub input_user_endpoint: String,
    pub input_user_export_endpoint: String,
    pub input_data_endpoint: String,
    pub pic_endpoint: String,
    pub input_pic_endpoint: String,
    pub pic_item_endpoint: String,
}
impl EnvConfig {
    pub fn load() -> Result<Self, EnvError> {
        dotenv::dotenv().ok();
        let config = Self {
            user_email: get_env_non_empty("USER_EMAIL")?,
            user_password: get_env_non_empty("USER_PASSWORD")?,
            base_url: get_env_url("BASE_URL")?,
            login_endpoint: get_env_endpoint("LOGIN_ENDPOINT")?,
            logout_endpoint: get_env_endpoint("LOGOUT_ENDPOINT")?,
            dashboard_endpoint: get_env_endpoint("DASHBOARD_ENDPOINT")?,
            cekunit_export_endpoint: get_env_endpoint("CEKUNIT_EXPORT_ENDPOINT")?,
            cekunit_unique_endpoint: get_env_endpoint("CEKUNIT_UNIQUE_ENDPOINT")?,
            cekunit_delete_category_endpoint: get_env_endpoint("CEKUNIT_DELETE_CATEGORY_ENDPOINT")?,
            delete_all_endpoint: get_env_endpoint("DELETE_ALL_ENDPOINT")?,
            cekunit_item_endpoint: get_env_endpoint("CEKUNIT_ITEM_ENDPOINT")?,
            input_user_endpoint: get_env_endpoint("INPUT_USER_ENDPOINT")?,
            input_user_export_endpoint: get_env_endpoint("INPUT_USER_EXPORT_ENDPOINT")?,
            input_data_endpoint: get_env_endpoint("INPUT_DATA_ENDPOINT")?,
            pic_endpoint: get_env_endpoint("PIC_ENDPOINT")?,
            input_pic_endpoint: get_env_endpoint("INPUT_PIC_ENDPOINT")?,
            pic_item_endpoint: get_env_endpoint("PIC_ITEM_ENDPOINT")?,
        };
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), EnvError> {
        if !self.user_email.contains('@') {
            return Err(EnvError::Invalid(
                "USER_EMAIL".into(),
                "harus mengandung karakter '@'".into(),
            ));
        }
        if self.user_password.len() < 8 {
            return Err(EnvError::Invalid(
                "USER_PASSWORD".into(),
                "minimal 8 karakter".into(),
            ));
        }
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(EnvError::InvalidUrl(
                "BASE_URL".into(),
                "harus diawali http:// atau https://".into(),
            ));
        }
        Ok(())
    }
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}/{}", self.base_url, endpoint)
    }
    pub fn full_login_url(&self) -> String {
        self.build_url(&self.login_endpoint)
    }
    pub fn full_logout_url(&self) -> String {
        self.build_url(&self.logout_endpoint)
    }
    pub fn full_dashboard_url(&self) -> String {
        self.build_url(&self.dashboard_endpoint)
    }
    pub fn full_cekunit_export_url(&self) -> String {
        self.build_url(&self.cekunit_export_endpoint)
    }
    pub fn full_cekunit_unique_url(&self) -> String {
        self.build_url(&self.cekunit_unique_endpoint)
    }
    pub fn full_cekunit_delete_category_url(&self) -> String {
        self.build_url(&self.cekunit_delete_category_endpoint)
    }
    pub fn full_delete_all_url(&self) -> String {
        self.build_url(&self.delete_all_endpoint)
    }
    pub fn full_cekunit_item_url(&self, no: &str) -> String {
        format!("{}/{}", self.build_url(&self.cekunit_item_endpoint), no)
    }
    pub fn full_input_user_url(&self) -> String {
        self.build_url(&self.input_user_endpoint)
    }
    pub fn full_input_user_export_url(&self) -> String {
        self.build_url(&self.input_user_export_endpoint)
    }
    pub fn full_input_data_url(&self) -> String {
        self.build_url(&self.input_data_endpoint)
    }
    pub fn full_pic_url(&self) -> String {
        self.build_url(&self.pic_endpoint)
    }
    pub fn full_input_pic_url(&self) -> String {
        self.build_url(&self.input_pic_endpoint)
    }
    pub fn full_pic_item_url(&self, id: &str) -> String {
        format!("{}/{}", self.build_url(&self.pic_item_endpoint), id)
    }
}
fn get_env_non_empty(key: &str) -> Result<String, EnvError> {
    let val = env::var(key).map_err(|_| EnvError::NotFound(key.to_string()))?;
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return Err(EnvError::Empty(key.to_string()));
    }
    Ok(trimmed.to_string())
}
fn get_env_url(key: &str) -> Result<String, EnvError> {
    let val = get_env_non_empty(key)?;
    if !val.starts_with("http://") && !val.starts_with("https://") {
        return Err(EnvError::InvalidUrl(
            key.to_string(),
            "harus diawali http:// atau https://".into(),
        ));
    }
    Ok(normalize_base(val))
}
fn get_env_endpoint(key: &str) -> Result<String, EnvError> {
    let val = get_env_non_empty(key)?;
    Ok(normalize_endpoint(val))
}
fn normalize_base(mut base: String) -> String {
    base = base.trim().to_string();
    if base.ends_with('/') {
        base.pop();
    }
    base
}
fn normalize_endpoint(mut endpoint: String) -> String {
    endpoint = endpoint.trim().to_string();
    endpoint = endpoint.trim_start_matches('/').to_string();
    endpoint
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    fn safe_remove_var(key: &str) {
        unsafe { env::remove_var(key) }
    }
    fn safe_set_var(key: &str, value: &str) {
        unsafe { env::set_var(key, value) }
    }
    fn setup() {
        safe_remove_var("USER_EMAIL");
        safe_remove_var("USER_PASSWORD");
        safe_remove_var("BASE_URL");
        safe_remove_var("LOGIN_ENDPOINT");
        safe_remove_var("LOGOUT_ENDPOINT");
        safe_remove_var("DASHBOARD_ENDPOINT");
        safe_remove_var("CEKUNIT_EXPORT_ENDPOINT");
        safe_remove_var("CEKUNIT_UNIQUE_ENDPOINT");
        safe_remove_var("CEKUNIT_DELETE_CATEGORY_ENDPOINT");
        safe_remove_var("DELETE_ALL_ENDPOINT");
        safe_remove_var("CEKUNIT_ITEM_ENDPOINT");
        safe_remove_var("INPUT_USER_ENDPOINT");
        safe_remove_var("INPUT_USER_EXPORT_ENDPOINT");
        safe_remove_var("INPUT_DATA_ENDPOINT");
        safe_remove_var("PIC_ENDPOINT");
        safe_remove_var("INPUT_PIC_ENDPOINT");
        safe_remove_var("PIC_ITEM_ENDPOINT");
    }
    #[test]
    fn test_missing_var() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "password123");
        safe_set_var("BASE_URL", "http://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::NotFound(_))));
    }
    #[test]
    fn test_empty_var() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "password123");
        safe_set_var("BASE_URL", "http://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        safe_set_var("DASHBOARD_ENDPOINT", "");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::Empty(_))));
    }
    #[test]
    fn test_invalid_url() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "password123");
        safe_set_var("BASE_URL", "ftp://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        safe_set_var("DASHBOARD_ENDPOINT", "dashboard");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::InvalidUrl(_, _))));
    }
    #[test]
    fn test_password_too_short() {
        setup();
        safe_set_var("USER_EMAIL", "test@example.com");
        safe_set_var("USER_PASSWORD", "123");
        safe_set_var("BASE_URL", "http://localhost");
        safe_set_var("LOGIN_ENDPOINT", "login");
        safe_set_var("LOGOUT_ENDPOINT", "logout");
        safe_set_var("DASHBOARD_ENDPOINT", "dashboard");
        safe_set_var("CEKUNIT_EXPORT_ENDPOINT", "export");
        safe_set_var("CEKUNIT_UNIQUE_ENDPOINT", "unique");
        safe_set_var("CEKUNIT_DELETE_CATEGORY_ENDPOINT", "delete_cat");
        safe_set_var("DELETE_ALL_ENDPOINT", "delete_all");
        safe_set_var("CEKUNIT_ITEM_ENDPOINT", "item");
        safe_set_var("INPUT_USER_ENDPOINT", "input_user");
        safe_set_var("INPUT_USER_EXPORT_ENDPOINT", "input_user_export");
        safe_set_var("INPUT_DATA_ENDPOINT", "input_data");
        safe_set_var("PIC_ENDPOINT", "pic");
        safe_set_var("INPUT_PIC_ENDPOINT", "input_pic");
        safe_set_var("PIC_ITEM_ENDPOINT", "pic_item");
        let result = EnvConfig::load();
        assert!(matches!(result, Err(EnvError::Invalid(_, _))));
    }
}
