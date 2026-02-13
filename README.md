# librcekunit

[![Crates.io](https://img.shields.io/crates/v/librcekunit.svg)](https://crates.io/crates/librcekunit)
[![Documentation](https://docs.rs/librcekunit/badge.svg)](https://docs.rs/librcekunit)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](#license)
[![CI Status](https://github.com/neuxdotdev/librcekunit/workflows/CI/badge.svg)](https://github.com/neuxdotdev/librcekunit/actions)

**librcekunit** is a pure Rust client library for the CekUnit admin panel. It provides a type‑safe, ergonomic API to interact with all major features of CekUnit, including authentication, dashboard management, input data (nasabah), PIC (Person In Charge) management, user management, and data export.

The library handles session persistence via a filesystem cache, CSRF token extraction, and retry logic with exponential backoff – so you can focus on your business logic.

## Features

- **Authentication** – Login with email/password, automatic CSRF token handling, session caching.
- **Dashboard** – Fetch paginated CekUnit lists, export data (Excel, PDF, CSV), get unique column values, delete records (single, by category, or all).
- **Input Data (Nasabah)** – Submit new customer records.
- **Input User** – List and export user‑input data with search, sort, and date filters.
- **PIC Management** – Create, update, delete, and list Persons In Charge.
- **User Management** – List and update application users.
- **Automatic Retries** – Configurable retry logic for transient failures (CSRF fetch, login, logout).
- **Session Cache** – Stores cookies and CSRF tokens in the system cache directory; no need to log in again on every run.
- **Environment‑based Configuration** – All endpoints and credentials are read from environment variables or a `.env` file.
- **Comprehensive Error Types** – Detailed error variants for every possible failure (network, authentication, CSRF, validation, etc.).
- **Logging** – Built‑in logging using the `log` crate; integrate with any logger (e.g., `env_logger`).

## Installation

You can add `librcekunit` to your project in multiple ways, depending on whether you want the latest released version or the development version.

---

### 1. Install via **Crates.io** (recommended)

Open terminal and run

```bash
cargo add librcekunit
```

Then build your project:

```bash
cargo build
```

This will fetch the latest stable release from [crates.io](https://crates.io/crates/librcekunit).

---

### 2. Install directly from **GitHub** (development / unreleased version)

If you want the latest commits or a specific branch/tag, add:

```toml
[dependencies]
librcekunit = { git = "https://github.com/neuxdotdev/librcekunit.git" }
```

Or use a specific tag:

```toml
[dependencies]
librcekunit = { git = "https://github.com/neuxdotdev/librcekunit.git", tag = "v1.2.0" }
```

Then build your project:

```bash
cargo build
```

> [!NOTE]
> Note: Using GitHub versions may include unstable changes. Prefer Crates.io for production.

## ️ Configuration

The library reads configuration from environment variables. You can also use a `.env` file (supported via `dotenv`).

### Required Variables

| Variable                           | Description                                                        |
| ---------------------------------- | ------------------------------------------------------------------ |
| `USER_EMAIL`                       | Your login email                                                   |
| `USER_PASSWORD`                    | Your login password (min. 8 characters)                            |
| `BASE_URL`                         | Base URL of the CekUnit installation (e.g., `https://example.com`) |
| `LOGIN_ENDPOINT`                   | Path to the login page (e.g., `login`)                             |
| `LOGOUT_ENDPOINT`                  | Path to logout                                                     |
| `DASHBOARD_ENDPOINT`               | Path to the main dashboard                                         |
| `CEKUNIT_EXPORT_ENDPOINT`          | Path for exporting CekUnit data                                    |
| `CEKUNIT_UNIQUE_ENDPOINT`          | Path for fetching unique column values                             |
| `CEKUNIT_DELETE_CATEGORY_ENDPOINT` | Path for deleting records by category                              |
| `DELETE_ALL_ENDPOINT`              | Path for deleting all CekUnit records                              |
| `CEKUNIT_ITEM_ENDPOINT`            | Path template for individual CekUnit items                         |
| `INPUT_USER_ENDPOINT`              | Path for input user listing                                        |
| `INPUT_USER_EXPORT_ENDPOINT`       | Path for exporting input user data                                 |
| `INPUT_DATA_ENDPOINT`              | Path for input data (nasabah) form                                 |
| `PIC_ENDPOINT`                     | Path for PIC listing                                               |
| `INPUT_PIC_ENDPOINT`               | Path for creating a new PIC                                        |
| `PIC_ITEM_ENDPOINT`                | Path template for individual PIC items                             |
| `USERS_ENDPOINT`                   | Path for users listing                                             |
| `USERS_ITEM_ENDPOINT`              | Path template for individual user items                            |

All endpoint paths must **not** start with a slash; they will be appended to `BASE_URL` automatically. Example `.env` file:

```ini
USER_EMAIL=admin@example.com
USER_PASSWORD=supersecret
BASE_URL=https://cekunit.example.com
LOGIN_ENDPOINT=login
LOGOUT_ENDPOINT=logout
DASHBOARD_ENDPOINT=dashboard
CEKUNIT_EXPORT_ENDPOINT=cekunit/export
...
```

## Quick Start

```rust,no_run
use librcekunit::CekUnitClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the main client – loads configuration from environment
    let mut client = CekUnitClient::new()?;

    // Log in (uses cached session if still valid)
    let session = client.login()?;
    println!("Logged in at: {}", session.timestamp);

    // Access the dashboard client
    let dashboard = client.dashboard()?;
    let html = dashboard.get_dashboard(Some(1), None, Some("created_at"), Some("desc"))?;
    println!("Dashboard page 1: {} bytes", html.len());

    // Log out when done
    client.logout()?;

    Ok(())
}
```

## Detailed Usage

### Main Client

[`CekUnitClient`] is the entry point. It manages the shared session cache and provides methods to obtain specialised sub‑clients.

```rust
let mut client = CekUnitClient::new()?;
client.login()?;                     // authenticate
let dashboard = client.dashboard()?;  // get dashboard client
let input_user = client.input_user()?;
let pic = client.pic()?;
let users = client.users()?;
client.logout()?;                     // terminate session
```

### Dashboard Operations

[`DashboardClient`] handles everything related to the main CekUnit list.

```rust
let dash = client.dashboard()?;

// Fetch the second page, sorted by name ascending
let html = dash.get_dashboard(Some(2), None, Some("name"), Some("asc"))?;

// Export all data as Excel
let excel = dash.export_cekunit("excel", "created_at", "desc")?;

// Get unique values for the "status" column
let statuses = dash.get_unique_values("status")?;

// Delete all records with status "rejected"
dash.delete_by_category("status", "rejected")?;

// Delete a single record
dash.delete_cekunit("123")?;

// Update a record
use std::collections::HashMap;
let mut updates = HashMap::new();
updates.insert("status", "approved");
dash.update_cekunit("123", updates)?;
```

### Input Data (Nasabah)

[`InputDataClient`] allows you to submit new nasabah records.

```rust
let input = client.input_data()?;

let mut data = HashMap::new();
data.insert("nama", "John Doe");
data.insert("alamat", "123 Main St");
data.insert("no_ktp", "1234567890");
input.insert_nasabah(data)?;
```

### Input User

[`InputUserClient`] provides listing and export of user‑input data.

```rust
let iu = client.input_user()?;

// Fetch first page, filter by search term, sort by date
let html = iu.get_input_user(
    Some(1),
    Some("john"),
    Some("created_at"),
    Some("desc"),
    Some("2025-01-01"),
    Some("2025-01-31")
)?;

// Export as PDF
let pdf = iu.export_input_user("pdf", "created_at", "desc", None, None, None)?;
```

### PIC Management

[`PicClient`] handles CRUD operations for Persons In Charge.

```rust
let pic = client.pic()?;

// List all PICs, sorted by name
let list = pic.get_pic_list(None, Some("name"), Some("asc"))?;

// Create a new PIC
let mut new = HashMap::new();
new.insert("name", "Jane Smith");
new.insert("email", "jane@example.com");
pic.insert_pic(new)?;

// Update an existing PIC
let mut updates = HashMap::new();
updates.insert("email", "new@example.com");
pic.update_pic("5", updates)?;

// Delete a PIC
pic.delete_pic("5")?;
```

### User Management

[`UsersClient`] allows you to list and update application users.

```rust
let users = client.users()?;

// List users, second page, sorted by email
let html = users.get_users_list(Some(2), Some("email"), Some("asc"))?;

// Update a user's details
let mut updates = HashMap::new();
updates.insert("name", "New Name");
users.update_user("42", updates)?;
```

## Session Management

Upon successful login, the client stores the session cookies and CSRF token in a JSON file inside the system’s cache directory (e.g., `~/.cache/librcekunit/` on Linux). Subsequent `CekUnitClient::new()` will automatically load this cache – you don’t need to log in again unless the session expires.

You can check the current session with `client.check_session()` and manually clear it with `client.logout()` or `client.auth_client().cache_manager().clear()`.

The cache is also automatically cleared after a successful logout.

## Error Handling

All methods return a [`Result<T, ApiError>`]. [`ApiError`] is an enum covering every possible failure:

- Network errors (timeout, connection refused)
- HTTP errors mapped to semantic variants (401 → `Unauthorized`, 419 → `CsrfExpired`, etc.)
- CSRF token not found
- Cache read/write errors
- Environment variable errors
- JSON parsing errors
- …

You can match on specific variants to handle different cases:

```rust
match client.login() {
    Ok(_) => println!("Logged in"),
    Err(ApiError::ValidationError(msg)) => eprintln!("Invalid credentials: {}", msg),
    Err(ApiError::CsrfTokenNotFound) => eprintln!("Login page structure changed"),
    Err(e) => eprintln!("Login failed: {}", e),
}
```

## Building and Testing

```bash
# Build the library
cargo build

# Run tests (requires a valid .env file with test credentials)
cargo test

# Generate documentation
cargo doc --open
```

## Documentation

Full API documentation is available at [docs.rs/librcekunit](https://docs.rs/librcekunit). You can also generate it locally with `cargo doc --open`.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on [GitHub](https://github.com/neuxdotdev/librcekunit).

## License

This project is licensed under the **GNU Affero General Public License v3.0** – see the [LICENSE](LICENSE) file for details.
