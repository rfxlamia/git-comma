mod ai;
mod config;
mod error;
mod openrouter;
mod preflight;
mod prompt;
mod sanitization;
mod setup;
mod tui;
mod ui;

use clap::Parser;
use config::{home_config_path, Config, ConfigError};

#[derive(Parser)]
#[command(name = "comma")]
#[command(version, about = "AI-powered git commit message generator")]
struct Cli {
    /// Run the interactive setup flow
    #[arg(long)]
    setup: bool,
}

fn fallback_editor() -> String {
    match crate::ai::open_editor("") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Editor error: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_commit_failure() {
    eprintln!("❌ Commit rejected by system (possibly pre-commit hook/linter failed).");
    eprintln!("💡 Draft is safe! After fixing, run:");
    eprintln!("   git commit -F .git/comma_msg.txt");
    std::process::exit(1);
}

fn run_git_add() -> bool {
    std::process::Command::new("git")
        .args(["add", "."])
        .spawn()
        .map(|mut child| child.wait().map(|e| e.success()).unwrap_or(false))
        .unwrap_or(false)
}

fn main() {
    let cli = Cli::parse();

    if cli.setup {
        match setup::run_setup_flow(false) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("❌ Setup failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    let config_path = home_config_path();
    let config = match Config::load_from_path(&config_path) {
        Ok(cfg) => cfg,
        Err(ConfigError::MalformedJson) => {
            eprintln!("Config corrupted. Deleting and re-setting up...");
            if let Err(e) = std::fs::remove_file(&config_path) {
                eprintln!("Warning: Failed to delete corrupted config: {}. Will overwrite.", e);
            }
            match setup::run_setup_flow(true) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("❌ Setup failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(_) if !config_path.exists() => {
            match setup::run_setup_flow(true) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("❌ Setup failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read config: {}. Re-running setup...", e);
            match setup::run_setup_flow(true) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("❌ Setup failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    // Pre-flight check
    let _preflight_result = match preflight::run() {
        Ok(success) => success,
        Err(preflight::PreflightError::WorkingTreeClean) => {
            println!("✨ Working tree clean.");
            std::process::exit(0);
        }
        Err(preflight::PreflightError::NotGitRepo) => {
            eprintln!("Error: This is not a git repository.");
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
                        Err(preflight::PreflightError::WorkingTreeClean) => {
                            println!("✨ Working tree clean.");
                            std::process::exit(0);
                        }
                        Err(preflight::PreflightError::NotGitRepo) => {
                            eprintln!("Error: This is not a git repository.");
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
            match ui::confirm_large_diff(size) {
                Ok(true) => {
                    match preflight::run_with_diff_bypass() {
                        Ok(success) => success,
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Ok(false) | Err(()) => {
                    eprintln!("\n❌ Cancelled. Stage fewer files and try again.");
                    std::process::exit(1);
                }
            }
        }
    };

    // Recovery Loop: obtain valid draft with bounded retry
    let mut working_config = config.clone();
    let mut attempt = 0;
    let max_attempts = 3;
    let mut draft = loop {
        attempt += 1;
        print!("⏳ Analyzing the diff and crafting the commit message...");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match crate::ai::run_ai_engine(
            &working_config.api_key,
            &working_config.model_id,
            &_preflight_result.diff_content,
        ) {
            Ok(d) => break d,
            Err(crate::ai::AiError::ModelUnavailable(ref msg)) | Err(crate::ai::AiError::RateLimitExceeded(ref msg)) => {
                if attempt >= max_attempts {
                    eprintln!("\n❌ {} after {} attempts.", msg, max_attempts);
                    eprintln!("💡 Continuing in manual editor mode...");
                    let content = fallback_editor();
                    break content;
                }
                match ui::prompt_model_switch(&working_config.model_id) {
                Ok(true) => {
                    match setup::run_setup_flow(false) {
                        Ok(new_config) => {
                            working_config = new_config;
                            continue;
                        }
                        Err(e) => {
                            eprintln!("\n⚠️ Setup failed: {}", e);
                            eprintln!("Re-entering setup flow...");
                            match setup::run_setup_flow(true) {
                                Ok(new_config) => {
                                    working_config = new_config;
                                    continue;
                                }
                                Err(e) => {
                                    eprintln!("❌ Setup failed: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                }
                Ok(false) | Err(()) => {
                    eprintln!("\n💡 Continuing in manual editor mode...");
                    let content = fallback_editor();
                    break content;
                }
            }
            }
            Err(crate::ai::AiError::Network(_)) => {
                eprintln!("\n❌ Network error. Continuing in manual editor mode...");
                let content = fallback_editor();
                break content;
            }
            Err(crate::ai::AiError::EmptyResponse) => {
                eprintln!("\n❌ Empty response from API. Continuing in manual editor mode...");
                let content = fallback_editor();
                break content;
            }
            Err(crate::ai::AiError::Api(_)) => {
                eprintln!("\n❌ Failed to contact OpenRouter. Continuing in manual editor mode...");
                let content = fallback_editor();
                break content;
            }
        }
    };

    // Lazy-save: persist new model if it changed
    if working_config.model_id != config.model_id {
        if let Err(e) = working_config.save(&config_path) {
            eprintln!("Warning: Failed to save new model config: {}", e);
        } else {
            println!("✅ Model successfully changed!");
        }
    }

    // Show result
    println!("\n==================================================");
    println!("{}", draft);
    println!("==================================================\n");

    // Action loop
    loop {
        match crate::ai::prompt_action(&draft) {
            Ok(crate::ai::Action::Accept) => {
                // Execute commit with draft
                if let Some(repo_root) = crate::ai::get_repo_root() {
                    match crate::ai::commit_with_draft(&draft, &repo_root) {
                        Ok(()) => {
                            println!("🎉 Commit successful!");
                            break;
                        }
                        Err(_e) => {
                            handle_commit_failure();
                        }
                    }
                } else {
                    eprintln!("❌ Could not find git repository root.");
                    std::process::exit(1);
                }
            }
            Ok(crate::ai::Action::Edit) => {
                match crate::ai::open_editor(&draft) {
                    Ok(edited) => {
                        if edited != draft {
                            println!("\n📝 Draft updated.");
                        }
                        // Execute commit with edited draft
                        if let Some(repo_root) = crate::ai::get_repo_root() {
                            match crate::ai::commit_with_draft(&edited, &repo_root) {
                                Ok(()) => {
                                    println!("🎉 Commit successful!");
                                    break;
                                }
                                Err(_e) => {
                                    handle_commit_failure();
                                }
                            }
                        } else {
                            eprintln!("❌ Could not find git repository root.");
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("Editor error: {}", e);
                    }
                }
            }
            Ok(crate::ai::Action::Regenerate) => match crate::ai::prompt_custom_instruction() {
                Ok(instruction) => {
                    match crate::ai::regenerate_with_instruction(
                        &working_config.api_key,
                        &working_config.model_id,
                        &_preflight_result.diff_content,
                        &instruction,
                    ) {
                        Ok(new_draft) => {
                            println!("\n==================================================");
                            println!("{}", new_draft);
                            println!("==================================================\n");
                            draft = new_draft;
                        }
                        Err(e) => {
                            eprintln!("\n❌ Regenerate failed: {}", e);
                        }
                    }
                }
                Err(_) => continue,
            },
            Ok(crate::ai::Action::Cancel) => {
                std::process::exit(0);
            }
            Err(_) => {
                std::process::exit(0);
            }
        }
    }
}
