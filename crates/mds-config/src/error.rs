use config::ConfigError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigLoadError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml_edit::TomlError),
    #[error("TOML edit error: {0}")]
    TomlEdit(#[from] toml::de::Error),
}
