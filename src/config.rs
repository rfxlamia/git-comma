use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub model_id: String,
}

impl Config {
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config =
            serde_json::from_str(&content).map_err(|_| ConfigError::MalformedJson)?;
        Ok(config)
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), ConfigError> {
        let tmp_path = path.with_extension("json.tmp");
        {
            let mut file = std::fs::File::create(&tmp_path)?;
            serde_json::to_writer_pretty(&mut file, self)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&tmp_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&tmp_path, perms)?;
        }

        std::fs::rename(&tmp_path, path)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    MalformedJson,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::MalformedJson => write!(f, "Malformed JSON"),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::IoError(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(_: serde_json::Error) -> Self {
        ConfigError::MalformedJson
    }
}

pub fn home_config_path() -> std::path::PathBuf {
    let home = home::home_dir().expect("Cannot find home directory");
    home.join(".comma.json")
}
