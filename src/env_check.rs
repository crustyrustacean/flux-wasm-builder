// src/env_check.rs

// dependencies
use std::process::Command;

/// Error type for environment check failures.
#[derive(Debug, thiserror::Error)]
pub enum EnvCheckError {
    #[error("wasm-pack not found on PATH — install from https://rustwasm.github.io/wasm-pack/")]
    WasmPackNotFound,

    #[error("wasm-pack version {found} is below minimum required {required}")]
    WasmPackVersionTooOld { found: String, required: String },

    #[error(
        "wasm32-unknown-unknown target not installed — run: rustup target add wasm32-unknown-unknown"
    )]
    Wasm32TargetMissing,

    #[error("failed to invoke '{command}': {source}")]
    SpawnFailed {
        command: String,
        source: std::io::Error,
    },

    #[error("failed to parse wasm-pack version output: {0}")]
    VersionParseFailed(String),
}

/// Check if `wasm-pack` is available on PATH.
pub fn wasm_pack_on_path() -> bool {
    let result = if cfg!(windows) {
        Command::new("where").arg("wasm-pack").output()
    } else {
        Command::new("which").arg("wasm-pack").output()
    };

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Check if `wasm-pack` version is at least the minimum required (0.13.0).
pub fn wasm_pack_version_ok() -> Result<(), EnvCheckError> {
    let output = Command::new("wasm-pack")
        .arg("--version")
        .output()
        .map_err(|e| EnvCheckError::SpawnFailed {
            command: "wasm-pack --version".into(),
            source: e,
        })?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let version_str = raw
        .trim()
        .strip_prefix("wasm-pack ")
        .unwrap_or_else(|| raw.trim());

    tracing::debug!(version = version_str, "wasm-pack version check");

    if version_meets_minimum(version_str, "0.13.0") {
        Ok(())
    } else {
        Err(EnvCheckError::WasmPackVersionTooOld {
            found: version_str.to_string(),
            required: "0.13.0".into(),
        })
    }
}

/// Check if the `wasm32-unknown-unknown` target is installed.
pub fn wasm32_target_installed() -> Result<(), EnvCheckError> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map_err(|e| EnvCheckError::SpawnFailed {
            command: "rustup target list --installed".into(),
            source: e,
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let installed = stdout.contains("wasm32-unknown-unknown");

    tracing::debug!(installed, "wasm32 target check");

    if installed {
        Ok(())
    } else {
        Err(EnvCheckError::Wasm32TargetMissing)
    }
}

/// Compare two semver-like version strings.
/// Returns true if `found` >= `minimum`.
fn version_meets_minimum(found: &str, minimum: &str) -> bool {
    fn parse(s: &str) -> Option<(u32, u32, u32)> {
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        Some((major, minor, patch))
    }
    match (parse(found), parse(minimum)) {
        (Some(f), Some(m)) => f >= m,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_meets_minimum_returns_true_for_equal_versions() {
        assert!(version_meets_minimum("0.13.0", "0.13.0"));
    }

    #[test]
    fn version_meets_minimum_returns_true_for_newer_versions() {
        assert!(version_meets_minimum("0.14.0", "0.13.0"));
        assert!(version_meets_minimum("1.0.0", "0.13.0"));
        assert!(version_meets_minimum("0.13.1", "0.13.0"));
    }

    #[test]
    fn version_meets_minimum_returns_false_for_older_versions() {
        assert!(!version_meets_minimum("0.12.0", "0.13.0"));
        assert!(!version_meets_minimum("0.13.0", "0.14.0"));
    }

    #[test]
    fn version_meets_minimum_handles_missing_patch() {
        assert!(version_meets_minimum("0.13", "0.13.0"));
        assert!(version_meets_minimum("1.0", "0.13.0"));
    }

    #[test]
    fn version_meets_minimum_returns_false_for_invalid_versions() {
        assert!(!version_meets_minimum("invalid", "0.13.0"));
        assert!(!version_meets_minimum("0.13.0", "invalid"));
    }
}
