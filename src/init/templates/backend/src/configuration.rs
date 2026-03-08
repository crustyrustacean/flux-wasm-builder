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
    /// Returns the port, respecting DRYDOCK_BACKEND_PORT override from the tool.
    pub fn effective_port(&self) -> u16 {
        std::env::var("DRYDOCK_BACKEND_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(self.port)
    }
}

/// Load configuration.
///
/// In development builds (no `embed-assets` feature), reads YAML files from
/// the filesystem, walking up from the current directory to find them.
///
/// In release builds (`embed-assets` feature), `configuration/base.yaml` is
/// embedded in the binary at compile time and used as the mandatory base.
/// An environment-specific YAML file is still read from the filesystem if
/// present next to the binary (using `required(false)`), allowing production
/// deployments to customise settings without a recompile. Environment
/// variables prefixed with `APP_` always override everything.
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    let environment_filename = format!("{}.yaml", environment.as_str());

    #[cfg(feature = "embed-assets")]
    {
        // Base config is embedded at compile time — binary works with zero
        // filesystem dependencies out of the box.
        let mut builder = config::Config::builder().add_source(config::File::from_str(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/configuration/base.yaml"
            )),
            config::FileFormat::Yaml,
        ));

        // If an environment-specific file exists alongside the binary (or
        // anywhere in the walk-up chain), layer it on top.  `required(false)`
        // means its absence is not an error.
        if let Some(base_path) = find_configuration_dir() {
            let env_file = base_path.join("configuration").join(&environment_filename);
            builder = builder.add_source(config::File::from(env_file).required(false));
        }

        builder = builder.add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        );

        return builder.build()?.try_deserialize::<Settings>();
    }

    #[cfg(not(feature = "embed-assets"))]
    {
        let base_path = find_configuration_dir().unwrap_or_else(|| {
            std::env::current_dir().expect("Failed to determine current directory")
        });

        let configuration_directory = base_path.join("configuration");

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
}

/// Walk up from the current directory looking for a `configuration/base.yaml`.
///
/// In dev mode this allows the backend to be launched from the workspace root
/// or from the backend crate directory directly.  In release mode it is used
/// to discover an optional environment-specific override file.
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
        let dir = tempdir().unwrap();
        unsafe {
            std::env::set_current_dir(dir.path()).unwrap();
        }
        let _ = find_configuration_dir();
    }

    #[test]
    fn effective_port_uses_drydock_backend_port_env_var() {
        unsafe {
            std::env::set_var("DRYDOCK_BACKEND_PORT", "9999");
        }
        let settings = ApplicationSettings {
            port: 3001,
            host: "127.0.0.1".to_string(),
            base_url: "http://127.0.0.1".to_string(),
        };
        assert_eq!(settings.effective_port(), 9999);
        unsafe {
            std::env::remove_var("DRYDOCK_BACKEND_PORT");
        }
    }

    #[test]
    fn effective_port_falls_back_to_config_port() {
        unsafe {
            std::env::remove_var("DRYDOCK_BACKEND_PORT");
        }
        let settings = ApplicationSettings {
            port: 3001,
            host: "127.0.0.1".to_string(),
            base_url: "http://127.0.0.1".to_string(),
        };
        assert_eq!(settings.effective_port(), 3001);
    }

    #[test]
    fn effective_port_ignores_invalid_env_var() {
        unsafe {
            std::env::set_var("DRYDOCK_BACKEND_PORT", "not-a-number");
        }
        let settings = ApplicationSettings {
            port: 3001,
            host: "127.0.0.1".to_string(),
            base_url: "http://127.0.0.1".to_string(),
        };
        assert_eq!(settings.effective_port(), 3001);
        unsafe {
            std::env::remove_var("DRYDOCK_BACKEND_PORT");
        }
    }
}
