// src/dev/mod.rs

// dependencies
use crate::domain::{ConfigError, FluxConfig};

#[derive(Debug, thiserror::Error)]
pub enum DevError {
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn run() -> Result<(), DevError> {
    let config_path = std::env::current_dir()?.join("flux.toml");
    let config = FluxConfig::from_file(&config_path)?;
    
    Ok(())
}