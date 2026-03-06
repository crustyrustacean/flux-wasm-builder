// src/dev/mod.rs

// dependencies
use crate::build::{BuildConfig, run_build_loop, run_wasm_pack, scss::compile_scss, start_watcher};
use crate::dev::server::serve;
use crate::domain::{ConfigError, FluxConfig};
use reqwest::Client;
use std::process::{Child, Command};
use std::sync::{Arc, RwLock};
use tokio::time::Duration;

// module declarations
pub mod proxy;
pub mod server;
pub mod styles;

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

    let styles_path = std::env::current_dir()?.join("frontend/styles");
    let initial_css = compile_scss(&styles_path).unwrap_or_default();
    let css_bytes: Arc<RwLock<Vec<u8>>> = Arc::new(RwLock::new(initial_css));

    let mut process_manager = ProcessManager::new(&config)?;
    ProcessManager::wait_for_ready(&config).await?;

    let (build_tx, build_rx) = tokio::sync::mpsc::channel::<()>(8);
    let (reload_tx, _reload_rx) = tokio::sync::broadcast::channel::<()>(16); // ← remove the underscore from _reload_rx
    let (restart_tx, mut restart_rx) = tokio::sync::mpsc::channel::<()>(8);

    let watch_path = std::env::current_dir()?.join("frontend/src");
    let _frontend_watcher = start_watcher(
        &watch_path,
        config.dev.watch_debounce_ms,
        build_tx,
        Some(&["rs"]),
    )?;

    let backend_watch_path = std::env::current_dir()?.join("backend/src");
    let _backend_watcher = start_watcher(
        &backend_watch_path,
        config.dev.watch_debounce_ms,
        restart_tx,
        Some(&["rs"]),
    )?;

    let (public_tx, mut public_rx) = tokio::sync::mpsc::channel::<()>(8);
    let public_watch_path = std::env::current_dir()?.join("frontend/public");
    let _public_watcher = start_watcher(
        &public_watch_path,
        config.dev.watch_debounce_ms,
        public_tx,
        None,
    )?;

    let reload_tx_for_public = reload_tx.clone();
    tokio::spawn(async move {
        while let Some(()) = public_rx.recv().await {
            let _ = reload_tx_for_public.send(());
        }
    });

    let (styles_tx, mut styles_rx) = tokio::sync::mpsc::channel::<()>(8);
    let styles_watch_path = std::env::current_dir()?.join("frontend/styles");
    let _styles_watcher = start_watcher(
        &styles_watch_path,
        config.dev.watch_debounce_ms,
        styles_tx,
        Some(&["scss"]),
    )?;

    let reload_tx_for_styles = reload_tx.clone();
    let css_bytes_for_styles = css_bytes.clone();
    tokio::spawn(async move {
        while let Some(()) = styles_rx.recv().await {
            let styles_path = std::env::current_dir()
                .expect("failed to get current dir")
                .join("frontend/styles");
            match compile_scss(&styles_path) {
                Ok(new_css) => {
                    *css_bytes_for_styles.write().unwrap() = new_css;
                    let _ = reload_tx_for_styles.send(());
                }
                Err(e) => {
                    tracing::warn!(error = %e, "SCSS compilation failed — previous styles still served");
                }
            }
        }
    });

    let reload_tx_clone = reload_tx.clone();
    let build_config = BuildConfig::new(std::env::current_dir()?.join("frontend"));
    println!("Building frontend...");
    run_wasm_pack(&build_config)
        .await
        .map_err(|_| DevError::Io(std::io::Error::other("initial wasm-pack build failed")))?;
    let build_config_for_serve = build_config.clone(); // ← clone before it moves into the spawn
    tokio::spawn(async move {
        run_build_loop(build_rx, reload_tx_clone, move || {
            let config = build_config.clone();
            async move { run_wasm_pack(&config).await }
        })
        .await;
    });

    let restart_config = config.clone();
    let reload_tx_for_restart = reload_tx.clone();
    tokio::spawn(async move {
        while let Some(()) = restart_rx.recv().await {
            if let Err(e) = process_manager.restart(&restart_config).await {
                eprintln!("Failed to restart backend: {e}");
            } else {
                let _ = reload_tx_for_restart.send(());
            }
        }
    });

    serve(&config, build_config_for_serve, reload_tx, css_bytes).await?;

    Ok(())
}

struct ProcessManager {
    handle: Option<Child>,
}

impl ProcessManager {
    fn new(config: &FluxConfig) -> Result<Self, DevError> {
        let backend_dir = std::env::current_dir()?.join("backend");
        let child = Command::new("cargo")
            .args(["run", "-p", &format!("{}-backend", config.project.name)])
            .env("FLUX_BACKEND_PORT", config.dev.backend_port.to_string())
            .current_dir(backend_dir)
            .spawn()?;

        Ok(Self {
            handle: Some(child),
        })
    }

    async fn wait_for_ready(config: &FluxConfig) -> Result<(), DevError> {
        let http_client = Client::new();
        let url = format!(
            "http://localhost:{}/api/health_check",
            config.dev.backend_port
        );

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

    async fn restart(&mut self, config: &FluxConfig) -> Result<(), DevError> {
        // Kill and reap the existing process
        if let Some(child) = self.handle.as_mut() {
            child.kill()?;
            child.wait()?;
        }

        // Respawn
        let backend_dir = std::env::current_dir()?.join("backend");
        let child = Command::new("cargo")
            .args(["run", "-p", &format!("{}-backend", config.project.name)])
            .env("FLUX_BACKEND_PORT", config.dev.backend_port.to_string())
            .current_dir(backend_dir)
            .spawn()?;

        self.handle = Some(child);

        // Wait for it to be ready again
        Self::wait_for_ready(config).await?;

        Ok(())
    }
}
