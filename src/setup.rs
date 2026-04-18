use crate::config::{home_config_path, Config};
use crate::openrouter::{ApiError, Client};
use crate::ui;

pub fn run_first_startup() -> Config {
    ui::welcome_message();

    let mut api_key = ui::api_key_prompt();

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

                break Config { api_key, model_id };
            }
            Err(ApiError::Unauthorized) => {
                ui::error_message("Invalid API Key. Make sure you entered the correct key.");
                api_key = ui::api_key_prompt();
            }
            Err(ApiError::Forbidden) => {
                ui::error_message(
                    "API Key doesn't have access. Check permissions on OpenRouter.",
                );
            }
            Err(ApiError::RateLimited) => {
                ui::rate_limited_message();
            }
            Err(ApiError::EmptyResponse) => {
                ui::error_message("No models available from OpenRouter. Please try again.");
            }
            Err(e) => {
                ui::error_message(&format!("Failed to fetch models: {}. Please try again.", e));
            }
        }
    };

    let path = home_config_path();
    config.save(&path).expect("Failed to save config");

    ui::save_confirmation();

    config
}
