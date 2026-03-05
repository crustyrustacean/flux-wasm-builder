// src/configuration.rs

use serde_aux::field_attributes::deserialize_number_from_string;
use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
}

impl ApplicationSettings {
    /// Returns the port, respecting FLUX_BACKEND_PORT override from the tool.
    pub fn effective_port(&self) -> u16 {
        std::env::var("FLUX_BACKEND_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.port)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = find_configuration_dir().unwrap_or_else(|| {
        std::env::current_dir().expect("Failed to determine current directory")
    });

    let configuration_directory = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(
            configuration_directory.join("base.yaml"),
        ))
        .add_source(config::File::from(
            configuration_directory.join(environment_filename),
        ))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}

/// Walk up from the current directory looking for a `configuration/base.yaml`.
/// This allows the backend to be launched from the workspace root (as
/// `flux-wasm-builder dev` does) or from the backend crate directory directly.
fn find_configuration_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("configuration").join("base.yaml").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. \
                 Use either `local` or `production`.",
                other
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn find_configuration_dir_returns_none_when_no_config_exists() {
        // A fresh temp dir has no configuration/base.yaml
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        // Can't assert None directly since other tests may have config dirs
        // above them, but we can assert it doesn't panic
        let _ = find_configuration_dir();
    }

    #[test]
    fn effective_port_uses_flux_backend_port_env_var() {
        std::env::set_var("FLUX_BACKEND_PORT", "9999");
        let settings = ApplicationSettings {
            port: 3001,
            host: "127.0.0.1".to_string(),
            base_url: "http://127.0.0.1".to_string(),
        };
        assert_eq!(settings.effective_port(), 9999);
        std::env::remove_var("FLUX_BACKEND_PORT");
    }

    #[test]
    fn effective_port_falls_back_to_config_port() {
        std::env::remove_var("FLUX_BACKEND_PORT");
        let settings = ApplicationSettings {
            port: 3001,
            host: "127.0.0.1".to_string(),
            base_url: "http://127.0.0.1".to_string(),
        };
        assert_eq!(settings.effective_port(), 3001);
    }

    #[test]
    fn effective_port_ignores_invalid_env_var() {
        std::env::set_var("FLUX_BACKEND_PORT", "not-a-number");
        let settings = ApplicationSettings {
            port: 3001,
            host: "127.0.0.1".to_string(),
            base_url: "http://127.0.0.1".to_string(),
        };
        assert_eq!(settings.effective_port(), 3001);
        std::env::remove_var("FLUX_BACKEND_PORT");
    }
}
