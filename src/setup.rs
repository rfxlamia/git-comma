use crate::config::{home_config_path, Config, ConfigError};
use crate::openrouter::{ApiError, Client};
use crate::ui;
use colored::Colorize;

fn print_splash_banner() {
    let banner = r#"
 ▄   ▄▄▄▄
 ▀██████▀                           ▄███████▄
   ██           ▄        ▄         ██   ▀█▄ ▀█
   ██     ▄███▄ ███▄███▄ ███▄███▄ ██  ▄█▀██  ██
   ██     ██ ██ ██ ██ ██ ██ ██ ██ ██  ██ ██ ▄█
   ▀█████▄▀███▀▄██ ██ ▀█▄██ ██ ▀█ ▀█▄  ▀▀▀▀▀▀
                                    ▀██████▀▀
"#;
    println!("{}", banner.cyan().bold());
}

fn prompt_api_key(is_first_run: bool, existing_key: Option<&str>) -> String {
    loop {
        let mut prompt_text = "Enter OpenRouter API Key (sk-or-v1-...):".to_string();
        if !is_first_run {
            prompt_text.push_str(" (Leave blank to keep existing)");
        }

        let password = inquire::Password::new(&prompt_text)
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .with_help_message("API key at https://openrouter.ai/keys")
            .prompt()
            .expect("User cancelled");

        if password.is_empty() {
            if is_first_run {
                ui::error_message("API key is required on first run. Please enter your key.");
                continue;
            } else {
                return existing_key
                    .expect("existing_key must be Some when is_first_run=false")
                    .to_string();
            }
        }
        break password;
    }
}

pub fn run_setup_flow(is_first_run: bool) -> Result<Config, ConfigError> {
    print_splash_banner();

    let existing_key = if !is_first_run {
        Config::load_from_path(&home_config_path())
            .ok()
            .map(|c| c.api_key)
    } else {
        None
    };

    let api_key = prompt_api_key(is_first_run, existing_key.as_deref());
    let mut client = Client::new(api_key.clone());

    let models = loop {
        ui::fetching_models_message();
        match client.fetch_models() {
            Ok(m) => break m,
            Err(ApiError::Unauthorized) => {
                ui::error_message("Invalid API Key. Make sure you entered the correct key.");
                let new_key = prompt_api_key(true, None);
                client = Client::new(new_key);
                continue;
            }
            Err(ApiError::RateLimited) => {
                ui::rate_limited_message();
                std::thread::sleep(std::time::Duration::from_secs(2));
                continue;
            }
            Err(ApiError::EmptyResponse) => {
                ui::error_message("No models available. Please try again.");
                continue;
            }
            Err(ApiError::Forbidden) => {
                return Err(ConfigError::ApiError(
                    "API key doesn't have access. Check permissions on OpenRouter.".into(),
                ));
            }
            Err(e) => {
                return Err(ConfigError::ApiError(format!(
                    "Failed to fetch models: {}. Please try again.",
                    e
                )));
            }
        }
    };

    let model_ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
    ui::models_loaded(model_ids.len());

    let selection = ui::model_select_prompt(&model_ids);
    let model_id = if selection == "[ Type Manual Model ID... ]" {
        ui::manual_model_prompt()
    } else {
        selection
    };

    let config = Config {
        api_key,
        model_id,
    };

    let path = home_config_path();
    if let Err(e) = config.save(&path) {
        eprintln!("Warning: Failed to save config: {}. Continuing anyway...", e);
    }

    ui::save_confirmation();
    Ok(config)
}
