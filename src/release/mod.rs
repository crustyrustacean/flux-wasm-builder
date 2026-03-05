// src/release/mod.rs

// dependencies
use crate::build::{BuildConfig, wasm_pack::run_wasm_pack};
use crate::domain::FluxConfig;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("configuration error: {0}")]
    Config(#[from] crate::domain::ConfigError),
    #[error("wasm-pack build failed")]
    WasmPackFailed,
    #[error("cargo build failed")]
    CargoFailed,
    #[error("scss compilation failed: {0}")]
    ScssFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn run() -> Result<(), ReleaseError> {
    let config_path = std::env::current_dir()?.join("flux.toml");
    let config = FluxConfig::from_file(&config_path)?;

    println!("Building frontend...");
    let build_config = BuildConfig::new(std::env::current_dir()?.join("frontend"));
    run_wasm_pack(&build_config)
        .await
        .map_err(|_| ReleaseError::WasmPackFailed)?;

    println!("Compiling styles...");
    let styles_path = std::env::current_dir()?.join("frontend/styles");
    let css = grass::from_path(
        styles_path.join("screen.scss"),
        &grass::Options::default(),
    ).map_err(|e| ReleaseError::ScssFailed(e.to_string()))?;
    std::fs::write(styles_path.join("screen.css"), css)?;

    println!("Building backend...");
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--features",
            "embed-assets",
            "-p",
            &format!("{}-backend", config.project.name),
        ])
        .status()?;

    if !status.success() {
        return Err(ReleaseError::CargoFailed);
    }

    println!(
        "Done. Binary at target/release/{}-backend",
        config.project.name
    );
    Ok(())
}
