use serde::Deserialize;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct Model {
    pub id: String,
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

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Forbidden => write!(f, "Forbidden"),
            ApiError::RateLimited => write!(f, "Rate limited"),
            ApiError::HttpError(code) => write!(f, "HTTP error {}", code),
            ApiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiError::ParseError => write!(f, "Parse error"),
            ApiError::EmptyResponse => write!(f, "Empty response"),
        }
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
        let mut resp = ureq::get("https://openrouter.ai/api/v1/models")
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .call()
            .map_err(|e: ureq::Error| match e {
                ureq::Error::StatusCode(code) => ApiError::HttpError(code),
                other => ApiError::NetworkError(other.to_string()),
            })?;

        let status_u16: u16 = resp.status().into();
        if status_u16 != 200 {
            match status_u16 {
                401 => return Err(ApiError::Unauthorized),
                403 => return Err(ApiError::Forbidden),
                429 => return Err(ApiError::RateLimited),
                code => return Err(ApiError::HttpError(code)),
            }
        }

        let models_resp: ModelsResponse = resp
            .body_mut()
            .read_json()
            .map_err(|_| ApiError::ParseError)?;

        if models_resp.data.is_empty() {
            return Err(ApiError::EmptyResponse);
        }

        Ok(models_resp.data)
    }

    /// Generate commit message from diff content via OpenRouter chat completions.
    pub fn generate_commit_message(&self, payload: &serde_json::Value) -> Result<String, ApiError> {
        let mut resp = ureq::post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send_json(payload)
            .map_err(|e: ureq::Error| match e {
                ureq::Error::StatusCode(code) => ApiError::HttpError(code),
                other => ApiError::NetworkError(other.to_string()),
            })?;

        let status_u16: u16 = resp.status().into();
        match status_u16 {
            200 => {}
            401 => return Err(ApiError::Unauthorized),
            403 => return Err(ApiError::Forbidden),
            429 => return Err(ApiError::RateLimited),
            code => return Err(ApiError::HttpError(code)),
        }

        #[derive(Deserialize)]
        struct MessageContent {
            content: Option<String>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: MessageContent,
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<Choice>,
        }

        let chat_resp: ChatResponse = resp
            .body_mut()
            .read_json()
            .map_err(|_| ApiError::ParseError)?;

        chat_resp
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .filter(|s| !s.trim().is_empty())
            .ok_or(ApiError::EmptyResponse)
    }
}
