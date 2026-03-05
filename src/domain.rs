// src/domain.rs

// dependencies
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read flux.toml: {source}")]
    ReadFailed {
        #[from]
        source: std::io::Error,
    },

    #[error("failed to parse flux.toml: {source}")]
    ParseFailed {
        #[from]
        source: toml::de::Error,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct FluxConfig {
    pub project: ProjectConfig,
    #[serde(default)]
    pub dev: DevConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DevConfig {
    pub public_port: u16,
    pub backend_port: u16,
    pub watch_debounce_ms: u64,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            public_port: 8080,
            backend_port: 3001,
            watch_debounce_ms: 300,
        }
    }
}

impl FluxConfig {
    pub fn from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let flux_config: FluxConfig = toml::from_str(&contents)?;

        Ok(flux_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn dev_config_defaults() {
        let config = DevConfig::default();
        assert_eq!(config.public_port, 8080);
        assert_eq!(config.backend_port, 3001);
        assert_eq!(config.watch_debounce_ms, 300);
    }

    #[test]
    fn flux_config_parses_valid_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("flux.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[project]
name = "test-app"

[dev]
public_port = 8080
backend_port = 3001
watch_debounce_ms = 300
"#
        )
        .unwrap();

        let config = FluxConfig::from_file(&config_path).unwrap();
        assert_eq!(config.project.name, "test-app");
        assert_eq!(config.dev.public_port, 8080);
    }

    #[test]
    fn flux_config_uses_dev_defaults() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("flux.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
[project]
name = "minimal"
"#
        )
        .unwrap();

        let config = FluxConfig::from_file(&config_path).unwrap();
        assert_eq!(config.dev.public_port, 8080); // default
        assert_eq!(config.dev.backend_port, 3001); // default
    }

    #[test]
    fn flux_config_missing_file_returns_error() {
        let result = FluxConfig::from_file(&PathBuf::from("/nonexistent/flux.toml"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::ReadFailed { .. }
        ));
    }

    #[test]
    fn flux_config_invalid_toml_returns_error() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("flux.toml");
        std::fs::write(&config_path, b"invalid [[toml").unwrap();

        let result = FluxConfig::from_file(&config_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::ParseFailed { .. }
        ));
    }

    #[test]
    fn flux_config_missing_project_name_returns_error() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("flux.toml");
        std::fs::write(&config_path, b"[dev]\npublic_port = 8080").unwrap();

        let result = FluxConfig::from_file(&config_path);
        assert!(result.is_err());
    }
}
