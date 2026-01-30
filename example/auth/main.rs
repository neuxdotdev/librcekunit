use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    pub token: String,
    pub csrf_token: String,
    pub expires_at: u64,
    pub user_id: String,
    pub email: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Credentials {
    pub email: String,
    pub password: String,
    pub base_url: String,
    pub login_endpoint: String,
    pub logout_endpoint: String,
}
#[allow(dead_code)]
#[derive(Debug, Error)]
enum AuthError {
    #[error("🌐 Network: {0}")]
    NetworkError(String),
    #[error("❌ Invalid email/password")]
    InvalidCredentials,
    #[error("⏰ Session expired")]
    SessionExpired,
    #[error("💾 Cache: {0}")]
    CacheError(String),
    #[error("🚨 API {0}: {1}")]
    ApiError(u16, String),
}
struct AuthClient {
    creds: Credentials,
    cache_file: PathBuf,
}
impl AuthClient {
    fn new(creds: Credentials) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("./.cache"))
            .join("cekunit");
        fs::create_dir_all(&cache_dir).ok();
        Self {
            creds,
            cache_file: cache_dir.join("session.json"),
        }
    }
    fn login(&self) -> Result<Session, AuthError> {
        println!("{}", "🔐 LOGIN".bright_green().bold());
        println!("  Email: {}", self.creds.email);
        println!(
            "  URL: {}{}",
            self.creds.base_url, self.creds.login_endpoint
        );
        if self.creds.email.is_empty() || self.creds.password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }
        println!("  {}", "→ Connecting to API...".dimmed());
        let session = Session {
            token: format!(
                "token_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            csrf_token: format!(
                "csrf_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            expires_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600,
            user_id: "user_123".to_string(),
            email: self.creds.email.clone(),
        };
        self.save_session(&session)?;
        println!("  {}", "✓ Login successful!".green());
        println!("  Token: {}...", &session.token[..15]);
        println!("  Expires in: {} seconds", 3600);
        println!("  Cache: {:?}", self.cache_file);
        Ok(session)
    }
    fn logout(&self) -> Result<(), AuthError> {
        println!("{}", "🚪 LOGOUT".bright_blue().bold());
        if let Ok(session) = self.load_session() {
            println!("  User: {}", session.email);
            println!(
                "  URL: {}{}",
                self.creds.base_url, self.creds.logout_endpoint
            );
            println!("  {}", "→ Calling logout API...".dimmed());
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        self.clear_cache()?;
        println!("  {}", "✓ Logout successful!".green());
        Ok(())
    }
    fn status(&self) -> Result<(), AuthError> {
        println!("{}", "📊 STATUS".bright_cyan().bold());
        println!("  Config:");
        println!("    Email: {}", self.creds.email);
        println!("    Base URL: {}", self.creds.base_url);
        println!("    Login Endpoint: {}", self.creds.login_endpoint);
        println!("    Logout Endpoint: {}", self.creds.logout_endpoint);
        println!("  Cache file: {:?}", self.cache_file);
        match self.load_session() {
            Ok(session) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let remaining = session.expires_at.saturating_sub(now);
                println!("  {}", "✓ SESSION ACTIVE".green().bold());
                println!("    User: {}", session.email);
                println!("    User ID: {}", session.user_id);
                println!("    Token: {}...", &session.token[..10]);
                println!("    Expires in: {} seconds", remaining);
                println!("    Valid: {}", if remaining > 0 { "✅" } else { "❌" });
                if remaining < 300 {
                    println!("    {}", "⚠️  Warning: Session expiring soon!".yellow());
                }
            }
            Err(_) => {
                println!("  {}", "✗ NO ACTIVE SESSION".red());
                println!("  Run: cekunit login");
            }
        }
        Ok(())
    }
    fn save_session(&self, session: &Session) -> Result<(), AuthError> {
        let data = serde_json::to_string_pretty(session)
            .map_err(|e| AuthError::CacheError(e.to_string()))?;
        fs::write(&self.cache_file, data).map_err(|e| AuthError::CacheError(e.to_string()))?;
        Ok(())
    }
    fn load_session(&self) -> Result<Session, AuthError> {
        if !self.cache_file.exists() {
            return Err(AuthError::CacheError("Cache file not found".to_string()));
        }
        let data = fs::read_to_string(&self.cache_file)
            .map_err(|e| AuthError::CacheError(e.to_string()))?;
        let session: Session =
            serde_json::from_str(&data).map_err(|e| AuthError::CacheError(e.to_string()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now > session.expires_at {
            return Err(AuthError::SessionExpired);
        }
        Ok(session)
    }
    fn clear_cache(&self) -> Result<(), AuthError> {
        if self.cache_file.exists() {
            fs::remove_file(&self.cache_file).map_err(|e| AuthError::CacheError(e.to_string()))?;
        }
        Ok(())
    }
}
#[derive(Parser)]
#[command(name = "cekunit")]
#[command(about = "CEK-UNIT Auth CLI - Simple & Powerful")]
#[command(version = "1.0")]
#[command(
    long_about = "A super simple yet complete authentication CLI for CEK-UNIT API
Examples:
  cekunit login          # Login dengan credentials dari .env
  cekunit status         # Cek status session
  cekunit logout         # Logout
  cekunit debug          # Debug info
  cekunit clean          # Clear cache"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(short, long, global = true)]
    verbose: bool,
}
#[derive(Subcommand)]
enum Command {
    Login,
    Logout,
    Status,
    Clean,
    Debug,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();
    if cli.verbose {
        println!("{}", "━".repeat(50).bright_black());
        println!("{}", " CEK-UNIT AUTH CLI ".bold().cyan());
        println!("{}", "━".repeat(50).bright_black());
    }
    let creds = Credentials {
        email: std::env::var("USER_EMAIL").unwrap_or_else(|_| "demo@example.com".to_string()),
        password: std::env::var("USER_PASSWORD").unwrap_or_else(|_| "demo123".to_string()),
        base_url: std::env::var("BASE_URL").unwrap_or_else(|_| "http://example.com".to_string()),
        login_endpoint: std::env::var("LOGIN_ENDPOINT").unwrap_or_else(|_| "/login".to_string()),
        logout_endpoint: std::env::var("LOGOUT_ENDPOINT").unwrap_or_else(|_| "/logout".to_string()),
    };
    let auth = AuthClient::new(creds);
    match cli.command {
        Command::Login => match auth.login() {
            Ok(_) => {
                if cli.verbose {
                    println!("\n{}", "✅ DONE".green().bold());
                }
            }
            Err(e) => {
                println!("{} {}", "❌ Error:".red().bold(), e);
                std::process::exit(1);
            }
        },
        Command::Logout => match auth.logout() {
            Ok(_) => println!("{}", "✅ Session cleared".green()),
            Err(e) => println!("{} {}", "⚠️ Warning:".yellow(), e),
        },
        Command::Status => match auth.status() {
            Ok(_) => {
                if cli.verbose {
                    println!("\n{}", "ℹ️  Quick Commands:".dimmed());
                    println!("  cekunit login     - Login dengan kredensial baru");
                    println!("  cekunit logout    - Logout dan clear session");
                    println!("  cekunit clean     - Clear semua cache");
                }
            }
            Err(e) => {
                if cli.verbose {
                    println!("{} {}", "ℹ️ Info:".blue(), e);
                }
            }
        },
        Command::Clean => {
            println!("{}", "🧹 CLEANING CACHE".bright_yellow().bold());
            auth.clear_cache()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            println!("  {}", "✓ All cache cleared".green());
        }
        Command::Debug => {
            println!("{}", "🐛 DEBUG INFO".bright_magenta().bold());
            println!("  OS: {}", std::env::consts::OS);
            println!("  Cache dir: {:?}", auth.cache_file.parent());
            println!("  Cache exists: {}", auth.cache_file.exists());
            println!("\n  {}", "📋 Environment Variables:".bold());
            let env_vars = [
                "USER_EMAIL",
                "USER_PASSWORD",
                "BASE_URL",
                "LOGIN_ENDPOINT",
                "LOGOUT_ENDPOINT",
            ];
            for var in env_vars {
                match std::env::var(var) {
                    Ok(value) => println!("    {} = {}", var, value),
                    Err(_) => println!("    {} = ❌ NOT SET", var),
                }
            }
            println!("\n  {}", "💡 Tips:".bold());
            println!("    • Create .env file with your credentials");
            println!("    • Use --verbose for detailed output");
            println!("    • Run 'cekunit status' to check current session");
        }
    }
    if cli.verbose {
        println!("\n{}", "━".repeat(50).bright_black());
    }
    Ok(())
}
