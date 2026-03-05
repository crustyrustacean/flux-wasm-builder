// src/build/build.rs

//! Core build functionality for invoking wasm-pack.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Error type for build failures.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// wasm-pack exited with a non-zero status.
    /// exit_code is None when the process was killed by a signal.
    #[error("wasm-pack failed (exit code: {exit_code:?})")]
    WasmPackFailed { exit_code: Option<i32> },

    /// Build timed out and the child process was killed.
    #[error("wasm-pack timed out after {secs}s")]
    Timeout { secs: u64 },

    /// The wasm-pack binary could not be found or spawned.
    #[error("failed to spawn wasm-pack: {source}")]
    SpawnFailed { source: std::io::Error },

    /// Waiting on the child process failed (OS error).
    #[error("failed to wait on wasm-pack process: {source}")]
    WaitFailed { source: std::io::Error },

    /// Killing the timed-out process failed. Build is still considered failed.
    #[error("failed to kill timed-out wasm-pack process: {source}")]
    KillFailed { source: std::io::Error },
}

/// Configuration for the build subsystem.
#[derive(Clone, Debug)]
pub struct BuildConfig {
    /// Path to the frontend crate directory (e.g., "../frontend")
    pub frontend_crate_path: PathBuf,
    /// Path to the pkg output directory (default: frontend_crate_path + "/pkg")
    pub pkg_output_path: PathBuf,
    /// Path to index.html (default: frontend_crate_path + "/index.html")
    pub index_html_path: PathBuf,
    /// Path to the public/ directory (default: frontend_crate_path + "/public")
    pub public_path: PathBuf,
    /// Watch debounce interval in milliseconds (default: 300)
    pub watch_debounce_ms: u64,
    /// WebSocket path for reload signaling (default: "/ws/reload")
    pub reload_ws_path: String,
    /// Server port (default: 8080)
    pub port: u16,
    /// Build timeout in seconds (default: 300)
    pub build_timeout_secs: u64,
}

impl BuildConfig {
    /// Create a new BuildConfig with the given frontend crate path.
    /// All other fields are set to reasonable defaults.
    pub fn new<P: Into<PathBuf>>(frontend_crate_path: P) -> Self {
        let frontend_crate_path = frontend_crate_path.into();
        Self {
            pkg_output_path: frontend_crate_path.join("pkg"),
            index_html_path: frontend_crate_path.join("index.html"),
            public_path: frontend_crate_path.join("public"),
            watch_debounce_ms: 300,
            reload_ws_path: "/ws/reload".to_string(),
            port: 8080,
            build_timeout_secs: 300,
            frontend_crate_path,
        }
    }
}

/// Run wasm-pack build with the given configuration.
pub async fn run_wasm_pack(config: &BuildConfig) -> Result<(), BuildError> {
    run_wasm_pack_with_env(config, &[]).await
}

/// Run wasm-pack build with additional environment variables.
///
/// This function is exposed for testing purposes, allowing tests to
/// shadow PATH without affecting the system environment.
pub async fn run_wasm_pack_with_env(
    config: &BuildConfig,
    extra_env: &[(&str, &str)],
) -> Result<(), BuildError> {
    let span = tracing::info_span!(
        "wasm_pack_build",
        frontend = %config.frontend_crate_path.display(),
        timeout_secs = config.build_timeout_secs,
    );
    let _enter = span.enter();

    tracing::info!("starting wasm-pack build");

    let mut cmd = Command::new("wasm-pack");
    cmd.args(["build", "--target", "web"])
        .arg(&config.frontend_crate_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Apply dev mode flag (in release builds, use --release)
    #[cfg(debug_assertions)]
    cmd.arg("--dev");
    #[cfg(not(debug_assertions))]
    cmd.arg("--release");

    for (k, v) in extra_env {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn().map_err(|e| {
        tracing::error!(error = %e, "failed to spawn wasm-pack");
        BuildError::SpawnFailed { source: e }
    })?;

    let timeout = Duration::from_secs(config.build_timeout_secs);
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) if status.success() => {
            tracing::info!("wasm-pack build succeeded");
            Ok(())
        }
        Ok(Ok(status)) => {
            let exit_code = status.code();
            tracing::error!(?exit_code, "wasm-pack build failed");
            Err(BuildError::WasmPackFailed { exit_code })
        }
        Ok(Err(e)) => {
            tracing::error!(error = %e, "error waiting on wasm-pack process");
            Err(BuildError::WaitFailed { source: e })
        }
        Err(_elapsed) => {
            tracing::error!(
                timeout_secs = config.build_timeout_secs,
                "wasm-pack timed out — killing process"
            );
            if let Err(e) = child.kill().await {
                tracing::warn!(error = %e, "failed to kill timed-out wasm-pack process");
                return Err(BuildError::KillFailed { source: e });
            }
            Err(BuildError::Timeout {
                secs: config.build_timeout_secs,
            })
        }
    }
}

/// Testable primitive for timeout behavior.
///
/// This function allows testing timeout behavior with any command,
/// not just wasm-pack.
pub async fn run_command_with_timeout(
    mut cmd: Command,
    timeout: Duration,
) -> Result<(), BuildError> {
    let mut child = cmd
        .spawn()
        .map_err(|e| BuildError::SpawnFailed { source: e })?;

    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(BuildError::WasmPackFailed {
            exit_code: status.code(),
        }),
        Ok(Err(e)) => Err(BuildError::WaitFailed { source: e }),
        Err(_elapsed) => {
            let _ = child.kill().await; // best-effort; process is already considered failed
            Err(BuildError::Timeout {
                secs: timeout.as_secs(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn build_config_default_public_path_is_relative_to_frontend() {
        let config = BuildConfig::new(PathBuf::from("../frontend"));
        assert_eq!(config.public_path, PathBuf::from("../frontend/public"));
    }

    #[test]
    fn build_config_default_pkg_path_is_relative_to_frontend() {
        let config = BuildConfig::new(PathBuf::from("../frontend"));
        assert_eq!(config.pkg_output_path, PathBuf::from("../frontend/pkg"));
    }

    #[test]
    fn build_config_default_index_html_path_is_relative_to_frontend() {
        let config = BuildConfig::new(PathBuf::from("../frontend"));
        assert_eq!(
            config.index_html_path,
            PathBuf::from("../frontend/index.html")
        );
    }

    #[test]
    fn build_config_default_timeout_is_300_seconds() {
        let config = BuildConfig::new("../frontend");
        assert_eq!(config.build_timeout_secs, 300);
    }

    #[test]
    fn build_config_default_port_is_8080() {
        let config = BuildConfig::new("../frontend");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn build_config_default_watch_debounce_is_300ms() {
        let config = BuildConfig::new("../frontend");
        assert_eq!(config.watch_debounce_ms, 300);
    }

    #[test]
    fn build_config_default_reload_ws_path() {
        let config = BuildConfig::new("../frontend");
        assert_eq!(config.reload_ws_path, "/ws/reload");
    }

    #[tokio::test]
    async fn returns_error_when_wasm_pack_not_on_path() {
        let config = BuildConfig::new("/nonexistent");
        let result = run_wasm_pack_with_env(&config, &[("PATH", "")]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn returns_wasm_pack_failed_for_invalid_crate_path() {
        let dir = tempdir().unwrap();
        let config = BuildConfig::new(dir.path());
        let result = run_wasm_pack(&config).await;
        assert!(matches!(result, Err(BuildError::WasmPackFailed { .. })));
    }

    #[tokio::test]
    async fn returns_timeout_error_when_process_hangs() {
        // Use a cross-platform long-running command
        // On Windows: timeout command waits for keypress, use ping instead
        // On Unix: sleep is available
        #[cfg(windows)]
        let cmd = {
            let mut c = Command::new("ping");
            c.args(["-n", "60", "127.0.0.1"]);
            c
        };
        #[cfg(not(windows))]
        let cmd = {
            let mut c = Command::new("sleep");
            c.arg("60");
            c
        };

        let result = run_command_with_timeout(cmd, Duration::from_millis(100)).await;
        assert!(matches!(result, Err(BuildError::Timeout { .. })));
    }

    #[tokio::test]
    async fn returns_success_for_fast_command() {
        // Use a cross-platform fast command
        // On Windows: cmd /c echo
        // On Unix: echo
        #[cfg(windows)]
        let cmd = {
            let mut c = Command::new("cmd");
            c.args(["/c", "echo", "test"]);
            c
        };
        #[cfg(not(windows))]
        let cmd = {
            let mut c = Command::new("echo");
            c.arg("test");
            c
        };

        let result = run_command_with_timeout(cmd, Duration::from_secs(5)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_error_for_failing_command() {
        // Use a cross-platform failing command
        // On Windows: cmd /c exit 1
        // On Unix: false
        #[cfg(windows)]
        let cmd = {
            let mut c = Command::new("cmd");
            c.args(["/c", "exit", "1"]);
            c
        };
        #[cfg(not(windows))]
        let cmd = {
            let c = Command::new("false");
            c
        };

        let result = run_command_with_timeout(cmd, Duration::from_secs(5)).await;
        assert!(matches!(
            result,
            Err(BuildError::WasmPackFailed { exit_code: Some(1) })
        ));
    }
}
