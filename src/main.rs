mod ai; // NEW
mod config;
mod error; // NEW: AI error types
mod openrouter;
mod preflight;
mod prompt; // NEW: AI prompt builder
mod sanitization; // NEW: response sanitization
mod setup;
mod tui; // NEW: AI TUI
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
                eprintln!("Config corrupted. Deleting and re-setting up...");
                std::fs::remove_file(&config_path).ok();
                setup::run_first_startup()
            }
            Err(e) => {
                eprintln!("Failed to read config: {}. Re-running setup...", e);
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
            if ui::confirm_large_diff(size) {
                match preflight::run_with_diff_bypass() {
                    Ok(success) => success,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("\n❌ Cancelled. Stage fewer files and try again.");
                std::process::exit(1);
            }
        }
    };

    // TODO: Pass preflight_result.diff_content to AI commit generation
    println!("AI working..."); // Placeholder

    // Run AI engine
    // Note: existing main.rs binds config as `_config`, so we use `_config`
    let mut draft = match crate::ai::run_ai_engine(
        &_config.api_key,
        &_config.model_id,
        &_preflight_result.diff_content,
    ) {
        Ok(draft) => draft,
        Err(crate::ai::AiError::Api(msg)) => {
            eprintln!("\n❌ Failed to contact OpenRouter ({})", msg);
            eprintln!("💡 Continue writing manually in Editor?");
            if inquire::Confirm::new("Open editor?")
                .with_default(true)
                .prompt()
                .unwrap_or(false)
            {
                match crate::ai::open_editor("") {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!("Editor error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                std::process::exit(1);
            }
        }
        Err(crate::ai::AiError::EmptyResponse) => {
            eprintln!("\n❌ Empty response from API. Try again or use the editor.");
            std::process::exit(1);
        }
        Err(crate::ai::AiError::Network(msg)) => {
            eprintln!("\n❌ Network error: {}", msg);
            std::process::exit(1);
        }
        Err(crate::ai::AiError::ModelUnavailable(msg)) => {
            eprintln!("\n❌ Model unavailable: {}", msg);
            std::process::exit(1);
        }
        Err(crate::ai::AiError::RateLimitExceeded(msg)) => {
            eprintln!("\n❌ Rate limit exceeded: {}", msg);
            std::process::exit(1);
        }
    };

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
                            eprintln!("❌ Commit rejected by system (possibly pre-commit hook/linter failed).");
                            eprintln!("💡 Draft is safe! After fixing, run:");
                            eprintln!("   git commit -F .git/comma_msg.txt");
                            std::process::exit(1);
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
                                    eprintln!("❌ Commit rejected by system (possibly pre-commit hook/linter failed).");
                                    eprintln!("💡 Draft is safe! After fixing, run:");
                                    eprintln!("   git commit -F .git/comma_msg.txt");
                                    std::process::exit(1);
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
                        &_config.api_key,
                        &_config.model_id,
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
