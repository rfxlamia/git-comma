use inquire::PasswordDisplayMode;

pub fn welcome_message() {
    println!();
    println!("============================================");
    println!("  Welcome to comma!");
    println!("  AI-powered git commit generator.");
    println!("============================================");
    println!();
    println!("First, we need a bit of configuration.");
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
    inquire::Password::new("Enter OpenRouter API Key (sk-or-v1-...):")
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_help_message("API key bisa diambil di https://openrouter.ai/keys")
        .prompt()
        .expect("User cancelled")
}

pub fn model_select_prompt(models: &[String]) -> String {
    use inquire::Select;

    let mut options: Vec<String> = vec!["[ Ketik Manual ID Model... ]".to_string()];
    options.extend(models.iter().cloned());

    let sel = Select::new("Select model (type to search):", options)
        .with_page_size(20)
        .prompt()
        .expect("User cancelled");

    sel
}

pub fn manual_model_prompt() -> String {
    inquire::Text::new("Enter model ID manually (e.g. anthropic/claude-3-haiku):")
        .prompt()
        .expect("User cancelled")
}

pub fn save_confirmation() {
    println!();
    println!("Configuration saved!");
    println!();
}

pub fn fetching_models_message() {
    print!("Fetching model list from OpenRouter...");
    std::io::Write::flush(&mut std::io::stdout()).ok();
}

pub fn models_loaded(count: usize) {
    println!(" done! {} models found.", count);
    println!("Type to search...");
    println!();
}

pub fn rate_limited_message() {
    error_message("Too many requests. Please wait a moment and try again.");
}

pub fn prompt_model_switch(model_name: &str) -> bool {
    println!();
    println!("❌ Oops! API Error");
    println!("The provider rejected the request for the '{}' model.", model_name);
    println!();
    inquire::Confirm::new("Do you want to change the AI model now?")
        .with_default(true)
        .prompt()
        .unwrap_or(false)
}

pub fn confirm_large_diff(size: usize) -> bool {
    println!();
    println!("⚠️ Diff too large ({} characters).", size);
    println!();
    println!("Committing with such a large diff is a Git anti-pattern:");
    println!("- You may have staged files that shouldn't be there (lock files, dist/)");
    println!("- Better to split into smaller commits per feature");
    println!();
    inquire::Confirm::new("Continue anyway?")
        .with_default(false)
        .prompt()
        .unwrap_or(false)
}

pub fn print_unstaged_files(files: &[crate::preflight::UnstagedFile]) {
    println!("⚠️ No files staged for commit.");
    println!();
    println!("Changed files that are not staged:");
    for file in files {
        println!(" {} {}", file.status, file.path);
    }
    println!();
}

pub fn prompt_git_add() -> bool {
    inquire::Confirm::new("Do you want to run 'git add .' now?")
        .with_default(true)
        .prompt()
        .unwrap_or(false)
}
