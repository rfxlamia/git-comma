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
            .map_err(|e: ureq::Error| ApiError::NetworkError(e.to_string()))?;

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
}
