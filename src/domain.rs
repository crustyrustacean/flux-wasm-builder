// src/domain.rs

// dependencies
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read flux.toml: {source}")]
    ReadFailed { #[from] source: std::io::Error },
    
    #[error("failed to parse flux.toml: {source}")]
    ParseFailed { #[from] source: toml::de::Error },
}

#[derive(Debug, Deserialize)]
pub struct FluxConfig {
    pub project: ProjectConfig,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}

impl FluxConfig {
    pub fn from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let flux_config: FluxConfig = toml::from_str(&contents)?;

        Ok(flux_config)
    }
}