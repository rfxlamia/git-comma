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
