mod config;
mod openrouter;
mod preflight;  // NEW
mod setup;
mod ui;

use config::{home_config_path, Config};

fn run_git_add() -> bool {
    std::process::Command::new("git")
        .args(["add", "."])
        .spawn()
        .map(|mut child| child.wait().map(|e| e.success()).unwrap_or(false))
        .unwrap_or(false)
}

fn main() {
    let config_path = home_config_path();

    let _config = if config_path.exists() {
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

    // Pre-flight check
    let _preflight_result = match preflight::run() {
        Ok(success) => success,
        Err(preflight::PreflightError::NotGitRepo) => {
            eprintln!("Error: Ini bukan git repository.");
            std::process::exit(1);
        }
        Err(preflight::PreflightError::GitCommandFailed { command, source }) => {
            eprintln!("Error: Git command '{}' failed: {}", command, source);
            std::process::exit(1);
        }
        Err(preflight::PreflightError::NoStagedFiles { unstaged }) => {
            ui::print_unstaged_files(&unstaged);
            if ui::prompt_git_add() {
                if run_git_add() {
                    // Re-run preflight check
                    match preflight::run() {
                        Ok(success) => success,
                        Err(preflight::PreflightError::NoStagedFiles { .. }) => {
                            eprintln!("Still no files staged after git add.");
                            std::process::exit(1);
                        }
                        Err(preflight::PreflightError::NotGitRepo) => {
                            eprintln!("Error: Ini bukan git repository.");
                            std::process::exit(1);
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("git add . failed.");
                    std::process::exit(1);
                }
            } else {
                std::process::exit(1);
            }
        }
        Err(preflight::PreflightError::DiffTooLarge { size }) => {
            if ui::confirm_large_diff(size) {
                match preflight::run_with_diff_bypass() {
                    Ok(success) => success,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                std::process::exit(1);
            }
        }
    };

    // TODO: Pass preflight_result.diff_content to AI commit generation
    println!("AI working...");  // Placeholder
}
