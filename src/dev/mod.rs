// src/dev/mod.rs

// dependencies
use crate::build::{BuildConfig, run_build_loop, run_wasm_pack, start_watcher};
use crate::domain::{ConfigError, FluxConfig};
use reqwest::Client;
use std::process::{Child, Command};
use tokio::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum DevError {
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("backend did not become ready in time")]
    BackendNotReady,

    #[error("file watcher error: {0}")]
    WatcherError(#[from] notify_debouncer_mini::notify::Error),
}

pub async fn run() -> Result<(), DevError> {
    let config_path = std::env::current_dir()?.join("flux.toml");
    let config = FluxConfig::from_file(&config_path)?;
    let _process_handle = ProcessManager::new(&config)?;
    ProcessManager::wait_for_ready(&config).await?;
    let (build_tx, build_rx) = tokio::sync::mpsc::channel::<()>(8);
    let (reload_tx, _reload_rx) = tokio::sync::broadcast::channel::<()>(16);
    let watch_path = std::env::current_dir()?.join("frontend/src");
    let _watcher = start_watcher(&watch_path, config.dev.watch_debounce_ms, build_tx)?;

    let reload_tx_clone = reload_tx.clone();
    let build_config = BuildConfig::new(std::env::current_dir()?.join("frontend"));
    tokio::spawn(async move {
        run_build_loop(build_rx, reload_tx_clone, move || {
            let config = build_config.clone();
            async move { run_wasm_pack(&config).await }
        })
        .await;
    });
    Ok(())
}

struct ProcessManager {
    handle: Option<Child>,
}

impl ProcessManager {
    fn new(config: &FluxConfig) -> Result<Self, DevError> {
        let child = Command::new("cargo")
            .args(["run", "-p", &format!("{}-backend", config.project.name)])
            .spawn()?;

        Ok(Self {
            handle: Some(child),
        })
    }

    async fn wait_for_ready(config: &FluxConfig) -> Result<(), DevError> {
        let http_client = Client::new();
        let url = format!("http://localhost:{}/api/health", config.dev.backend_port);

        let max_attempts = 30;
        let mut attempts = 0;

        loop {
            if attempts >= max_attempts {
                return Err(DevError::BackendNotReady);
            }

            match http_client.get(&url).send().await {
                Ok(response) if response.status() == 200 => return Ok(()),
                _ => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }

            attempts += 1;
        }
    }
}
