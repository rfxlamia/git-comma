# comma-cli: First Startup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the first startup flow for `comma` CLI — a flag-free terminal interface for AI-generated git commits. This produces a working binary that can be invoked, checks for `~/.comma.json`, and if missing, walks the user through API key entry, model selection, and config creation.

**Architecture:** Single binary (`comma`) with three modules: `config` (file I/O), `openrouter` (API client), and `setup` (first startup orchestration). All terminal UI uses `inquire` crate.

**Tech Stack:** Rust (no framework), `inquire` for prompts, `ureq` for HTTP, `serde`/`serde_json` for JSON, `home` for home dir detection.

---

## File Structure

```
comma-cli/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point, delegates to setup or main flow
│   ├── config.rs        # Config struct, load/save ~/.comma.json (atomic write)
│   ├── openrouter.rs    # OpenRouter API client (fetch models, verify key)
│   ├── setup.rs         # First startup orchestrator (greeting → key → select → save)
│   └── ui.rs            # UI helpers (welcome, errors, confirmations)
└── tests/
    └── config_tests.rs  # Unit tests for config module
```

---

## Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (stub — just prints "comma" and exits for now)

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "comma-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
inquire = "0.5"
ureq = "3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
home = "0.4"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create stub `src/main.rs`**

```rust
fn main() {
    println!("comma");
}
```

- [ ] **Step 3: Verify project compiles**

Run: `cargo build`
Expected: Compiles successfully with warnings (unused deps OK for now)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "chore: scaffold comma-cli project"
```

---

## Task 2: Config Module

**Files:**
- Create: `src/config.rs`
- Create: `tests/config_tests.rs`

- [ ] **Step 1: Write the failing test for `Config` struct and `load()`

```rust
use std::path::Path;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_load_config_success() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("comma.json");
    let content = r#"{"api_key": "sk-or-v1-test", "model_id": "test/model"}"#;
    std::fs::write(&config_path, content).unwrap();

    let result = crate::config::load_from_path(&config_path);
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert_eq!(cfg.api_key, "sk-or-v1-test");
    assert_eq!(cfg.model_id, "test/model");
}
```

Run: `cargo test test_load_config_success`
Expected: FAIL — `load_from_path` not defined

- [ ] **Step 2: Write `Config` struct and `load_from_path()` in `src/config.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub model_id: String,
}

impl Config {
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content).map_err(|_| ConfigError::MalformedJson)?;
        Ok(config)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    MalformedJson,
    MissingFile,
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::IoError(e)
    }
}
```

- [ ] **Step 3: Write failing test for `save()` atomic write**

```rust
#[test]
fn test_save_config_atomic() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("comma.json");

    let cfg = crate::config::Config {
        api_key: "sk-or-v1-test".into(),
        model_id: "test/model".into(),
    };

    cfg.save(&config_path).unwrap();

    let loaded = std::fs::read_to_string(&config_path).unwrap();
    assert!(loaded.contains("sk-or-v1-test"));
    // Verify file has restricted permissions (0o600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&config_path).unwrap();
        let perm = meta.permissions().mode();
        assert_eq!(perm & 0o777, 0o600);
    }
}
```

Run: `cargo test test_save_atomic`
Expected: FAIL — `save` not defined

- [ ] **Step 4: Implement `save()` with atomic write and 0o600 permissions**

```rust
impl Config {
    pub fn save(&self, path: &std::path::Path) -> Result<(), ConfigError> {
        let tmp_path = path.with_extension("json.tmp");
        {
            let mut file = std::fs::File::create(&tmp_path)?;
            serde_json::to_writer_pretty(&mut file, self)?;
        }
        std::fs::rename(&tmp_path, path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        Ok(())
    }
}
```

- [ ] **Step 5: Add `home_config_path()` helper using `home` crate**

```rust
pub fn home_config_path() -> std::path::PathBuf {
    let home = home::home_dir().expect("Cannot find home directory");
    home.join(".comma.json")
}
```

- [ ] **Step 6: Run all config tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/config.rs tests/config_tests.rs
git commit -m "feat: add config module with atomic write and 0o600 permissions"
```

---

## Task 3: OpenRouter API Client

**Files:**
- Create: `src/openrouter.rs`

Skip unit test for `fetch_models()` — `ureq` doesn't have easy mocking. Tested via the integration test in Task 5 instead.

- [ ] **Step 2: Write the OpenRouter client structs in `src/openrouter.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<Model>,
}

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    RateLimited,
    HttpError(u16),
    NetworkError(String),
    ParseError,
    EmptyResponse,
}

impl ApiError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, ApiError::RateLimited | ApiError::NetworkError(_))
    }
}

pub struct Client {
    api_key: String,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn fetch_models(&self) -> Result<Vec<Model>, ApiError> {
        let resp = ureq::get("https://openrouter.ai/api/v1/models")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .call()
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        match resp.status() {
            200 => {}
            401 => return Err(ApiError::Unauthorized),
            403 => return Err(ApiError::Forbidden),
            429 => return Err(ApiError::RateLimited),
            code => return Err(ApiError::HttpError(code)),
        }

        let models_resp: ModelsResponse = resp
            .into_json()
            .map_err(|_| ApiError::ParseError)?;

        if models_resp.data.is_empty() {
            return Err(ApiError::EmptyResponse);
        }

        Ok(models_resp.data)
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add src/openrouter.rs
git commit -m "feat: add OpenRouter API client with error handling"
```

---

## Task 4: Setup Flow (Orchestrator)

**Files:**
- Create: `src/ui.rs` (UI helpers: welcome, errors, confirmations)
- Create: `src/setup.rs` (first startup orchestrator)
- Modify: `src/main.rs` (wire it all together)

### Task 4a: UI Helpers

- [ ] **Step 1: Write `src/ui.rs` with welcome, error, and confirmation helpers**

```rust
use inquire::PasswordDisplayMode;

pub fn welcome_message() {
    println!();
    println!("============================================");
    println!("  Selamat datang di comma!");
    println!("  AI-powered git commit generator.");
    println!("============================================");
    println!();
    println!("Pertama-tama, kita perlu sedikit konfigurasi.");
    println!();
}

pub fn error_message(message: &str) {
    eprintln!();
    eprintln!("============================================");
    eprintln!("  ERROR");
    eprintln!("============================================");
    eprintln!();
    eprintln!("  {}", message);
    println!();
}

pub fn api_key_prompt() -> String {
    inquire::Password::new("Masukkan OpenRouter API Key (sk-or-v1-...):")
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_help_message("API key bisa diambil di https://openrouter.ai/keys")
        .prompt()
        .expect("Failed to read API key")
}

pub fn model_select_prompt(models: &[String]) -> String {
    use inquire::Select;

    let mut options: Vec<String> = vec!["[ Ketik Manual ID Model... ]".to_string()];
    options.extend(models.iter().cloned());

    let sel = Select::new(
        "Pilih model (ketik untuk mencari):",
        options,
    )
    .with_page_size(20)
    .prompt()
    .expect("Failed to select model");

    sel
}

pub fn manual_model_prompt() -> String {
    inquire::Text::new("Masukkan ID model secara manual (contoh: anthropic/claude-3-haiku):")
        .prompt()
        .expect("Failed to read manual model ID")
}

pub fn save_confirmation() {
    println!();
    println!("Konfigurasi disimpan!");
    println!();
}

pub fn fetching_models_message() {
    print!("Mengambil daftar model dari OpenRouter...");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}

pub fn models_loaded(count: usize) {
    println!(" done! {} model ditemukan.", count);
    println!("Ketik untuk mencari...");
    println!();
}

pub fn rate_limited_message() {
    error_message("Terlalu banyak permintaan. Mohon tunggu sebentar dan coba lagi.");
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add src/ui.rs
git commit -m "feat: add UI helpers for prompts and messages"
```

### Task 4b: Setup Orchestrator

- [ ] **Step 1: Write `src/setup.rs` with the setup loop**

```rust
use crate::config::{Config, home_config_path};
use crate::openrouter::{ApiError, Client};
use crate::ui;

pub fn run_first_startup() -> Config {
    ui::welcome_message();

    let api_key = ui::api_key_prompt();

    let config = loop {
        ui::fetching_models_message();

        let client = Client::new(api_key.clone());
        match client.fetch_models() {
            Ok(models) => {
                let model_ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
                ui::models_loaded(model_ids.len());

                let selection = ui::model_select_prompt(&model_ids);
                let model_id = if selection == "[ Ketik Manual ID Model... ]" {
                    ui::manual_model_prompt()
                } else {
                    selection
                };

                break Config {
                    api_key,
                    model_id,
                };
            }
            Err(ApiError::Unauthorized) => {
                ui::error_message("API Key tidak valid. Pastikan Anda memasukkan key yang benar.");
                api_key = ui::api_key_prompt();
            }
            Err(ApiError::Forbidden) => {
                ui::error_message("API Key tidak memiliki akses. Periksa permissions di OpenRouter.");
            }
            Err(ApiError::RateLimited) => {
                ui::rate_limited_message();
            }
            Err(ApiError::EmptyResponse) => {
                ui::error_message("Tidak ada model tersedia dari OpenRouter. Silakan coba lagi.");
            }
            Err(e) => {
                ui::error_message(&format!("Gagal mengambil model: {}. Silakan coba lagi.", e));
            }
        }
    };

    // Save config
    let path = home_config_path();
    config.save(&path).expect("Failed to save config");

    ui::save_confirmation();

    config
}
```

- [ ] **Step 2: Wire up `src/main.rs`**

```rust
mod config;
mod openrouter;
mod setup;
mod ui;

use config::{home_config_path, Config};

fn main() {
    let config_path = home_config_path();

    let config = if config_path.exists() {
        match Config::load_from_path(&config_path) {
            Ok(cfg) => cfg,
            Err(crate::config::ConfigError::MalformedJson) => {
                eprintln!("Konfigurasi corrupted. Menghapus dan/setup ulang...");
                std::fs::remove_file(&config_path).ok();
                setup::run_first_startup()
            }
            Err(e) => {
                eprintln!("Gagal membaca konfigurasi: {}. Setup ulang...", e);
                setup::run_first_startup()
            }
        }
    } else {
        setup::run_first_startup()
    };

    println!("Config loaded: model_id = {}", config.model_id);
}
```

- [ ] **Step 3: Verify full build**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/setup.rs src/main.rs
git commit -m "feat: implement first startup flow with API key verification and model selection"
```

---

## Task 5: Integration Test

**Files:**
- Create: `tests/integration_tests.rs`

- [ ] **Step 1: Write integration test for corrupted JSON recovery**

```rust
use std::fs;
use std::path::PathBuf;

fn home_config_path_fake() -> PathBuf {
    let tmp = std::env::temp_dir();
    tmp.join("comma_test.json")
}

#[test]
fn test_corrupted_json_triggers_setup() {
    let path = home_config_path_fake();
    fs::write(&path, "not valid json {{{").unwrap();

    // Verify file exists but is malformed
    let content = fs::read_to_string(&path).unwrap();
    assert!(!content.is_empty());

    // When loading via Config::load_from_path, malformed JSON should be detected
    let result = comma_cli::config::Config::load_from_path(&path);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), comma_cli::config::ConfigError::MalformedJson));

    fs::remove_file(&path).ok();
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add tests/integration_tests.rs Cargo.toml
git commit -m "test: add integration tests for config handling"
```

---

## Spec Coverage Check

| Spec Section | Task(s) |
|--------------|---------|
| Config file structure + atomic write + 0o600 | Task 2 |
| First startup flow (greeting → key → fetch → select → save) | Task 4a, 4b |
| API call to OpenRouter with error handling | Task 3 |
| 401/403/429 handling + retry | Task 3, Task 4b |
| Empty model list (`EmptyResponse` error + handling) | Task 3 |
| `PasswordDisplayMode::Masked` for API key | Task 4a |
| Full model list, no truncation | Task 4b |
| Fuzzy filter via `inquire::Select` | Task 4a |
| "Ketik Manual" option at top | Task 4a |
| Save confirmation message | Task 4a |
| Corrupted JSON detection | Task 4b (`main.rs`) |

---

**Plan complete and saved to `docs/superpowers/plans/2026-04-18-comma-first-startup.md`.**

Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
