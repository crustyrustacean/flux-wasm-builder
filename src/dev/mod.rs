//! Development server module
//!
//! This module provides the core development server functionality for flux-wasm-builder.
//! It orchestrates the entire development loop including:
//!
//! - Building the frontend with `wasm-pack`
//! - Spawning and managing the backend process
//! - File watching for hot-reloading
//! - SCSS compilation
//! - WebSocket-based browser reload signaling
//!
//! # Example
//!
//! ```ignore
//! use flux_wasm_builder::dev;
//!
//! // Start the dev server without opening a browser
//! dev::run(false).await?;
//!
//! // Start the dev server and open browser automatically
//! dev::run(true).await?;
//! ```

// dependencies
use crate::build::{
    BuildConfig, FileWatcher, run_build_loop, run_wasm_pack, scss::compile_scss, start_watcher,
};
use crate::dev::server::serve;
use crate::domain::{ConfigError, FluxConfig, WatchAction};
use reqwest::Client;
use std::process::{Child, Command};
use std::sync::{Arc, RwLock};
use tokio::time::Duration;

// module declarations
pub mod proxy;
pub mod server;
pub mod styles;

/// Errors that can occur during development server operation
#[derive(Debug, thiserror::Error)]
pub enum DevError {
    /// Configuration file could not be read or parsed
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// I/O error (file system, process spawning, etc.)
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Backend process did not respond to health checks within timeout
    #[error("backend did not become ready in time")]
    BackendNotReady,

    /// File watcher encountered an error
    #[error("file watcher error: {0}")]
    WatcherError(#[from] notify_debouncer_mini::notify::Error),
}

/// Start the development server
///
/// This function orchestrates the entire development loop:
///
/// 1. Loads configuration from `flux.toml` in the current directory
/// 2. Compiles initial SCSS from `frontend/styles/`
/// 3. Spawns the backend process and waits for it to be ready
/// 4. Sets up file watchers for frontend, backend, styles, and public assets
/// 5. Performs initial WASM build with `wasm-pack`
/// 6. Optionally opens a browser window
/// 7. Starts the HTTP server with proxy, WebSocket, and static file handling
///
/// # Arguments
///
/// * `open_browser` - If `true`, opens the default browser at the dev server URL
///   (`http://127.0.0.1:{public_port}`) after the server is ready. Browser opening
///   failures are logged as warnings and do not block server startup.
///
/// # Errors
///
/// Returns `DevError` if:
/// - Configuration cannot be loaded
/// - Backend process cannot be spawned or doesn't become ready
/// - File watchers cannot be initialized
/// - The HTTP server fails to bind or encounters a fatal error
///
/// # Example
///
/// ```ignore
/// // Start dev server without opening browser
/// dev::run(false).await?;
///
/// // Start dev server and open browser automatically
/// dev::run(true).await?;
/// ```
pub async fn run(open_browser: bool) -> Result<(), DevError> {
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

    // Collect all watcher handles so they don't get dropped
    let mut watchers: Vec<FileWatcher> = Vec::new();

    // Core watcher: frontend/src → WASM rebuild
    let watch_path = std::env::current_dir()?.join("frontend/src");
    watchers.push(start_watcher(
        &watch_path,
        config.dev.watch_debounce_ms,
        build_tx.clone(),
        Some(&["rs"]),
    )?);

    // Core watcher: backend/src → Backend restart
    let backend_watch_path = std::env::current_dir()?.join("backend/src");
    watchers.push(start_watcher(
        &backend_watch_path,
        config.dev.watch_debounce_ms,
        restart_tx.clone(),
        Some(&["rs"]),
    )?);

    // Core watcher: frontend/public → Page reload
    let (public_tx, mut public_rx) = tokio::sync::mpsc::channel::<()>(8);
    let public_watch_path = std::env::current_dir()?.join("frontend/public");
    watchers.push(start_watcher(
        &public_watch_path,
        config.dev.watch_debounce_ms,
        public_tx,
        None,
    )?);

    let reload_tx_for_public = reload_tx.clone();
    tokio::spawn(async move {
        while let Some(()) = public_rx.recv().await {
            let _ = reload_tx_for_public.send(());
        }
    });

    // Core watcher: frontend/styles → SCSS recompilation
    let (styles_tx, mut styles_rx) = tokio::sync::mpsc::channel::<()>(8);
    let styles_watch_path = std::env::current_dir()?.join("frontend/styles");
    watchers.push(start_watcher(
        &styles_watch_path,
        config.dev.watch_debounce_ms,
        styles_tx,
        Some(&["scss"]),
    )?);

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

    // Custom watchers from configuration
    // Clone the watch configs to avoid lifetime issues with the loop
    let custom_watches: Vec<_> = config.watch.clone();
    for watch_config in custom_watches {
        let watch_path = std::env::current_dir()?.join(&watch_config.path);

        // Skip if path doesn't exist
        if !watch_path.exists() {
            tracing::warn!(
                path = %watch_path.display(),
                "watch path does not exist, skipping"
            );
            continue;
        }

        // Determine extensions slice - leak memory for 'static lifetime (acceptable for long-running process)
        let extensions: Option<&'static [&'static str]> = if watch_config.extensions.is_empty() {
            None
        } else {
            // Leak each string and the slice - acceptable since watchers live for process lifetime
            let exts: Vec<&'static str> = watch_config
                .extensions
                .iter()
                .map(|s| {
                    let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
                    leaked
                })
                .collect();
            let boxed: Box<[&'static str]> = exts.into_boxed_slice();
            Some(Box::leak(boxed))
        };

        let debounce_ms = config.dev.watch_debounce_ms;

        // Create channel and spawn handler based on action type
        match watch_config.action {
            WatchAction::Reload => {
                let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(8);
                let reload = reload_tx.clone();
                let path_for_log = watch_config.path.display().to_string();
                let exts_for_log = watch_config.extensions.clone();
                watchers.push(start_watcher(&watch_path, debounce_ms, tx, extensions)?);
                tokio::spawn(async move {
                    while let Some(()) = rx.recv().await {
                        let _ = reload.send(());
                    }
                });
                tracing::info!(
                    path = %path_for_log,
                    extensions = ?exts_for_log,
                    "custom reload watcher started"
                );
            }
            WatchAction::Rebuild => {
                let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(8);
                let reload = reload_tx.clone();
                let build = build_tx.clone();
                let path_for_log = watch_config.path.display().to_string();
                let exts_for_log = watch_config.extensions.clone();
                watchers.push(start_watcher(&watch_path, debounce_ms, tx, extensions)?);
                tokio::spawn(async move {
                    while let Some(()) = rx.recv().await {
                        let _ = build.send(()).await;
                        let _ = reload.send(());
                    }
                });
                tracing::info!(
                    path = %path_for_log,
                    extensions = ?exts_for_log,
                    "custom rebuild watcher started"
                );
            }
            WatchAction::Restart => {
                let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(8);
                let restart = restart_tx.clone();
                let path_for_log = watch_config.path.display().to_string();
                let exts_for_log = watch_config.extensions.clone();
                watchers.push(start_watcher(&watch_path, debounce_ms, tx, extensions)?);
                tokio::spawn(async move {
                    while let Some(()) = rx.recv().await {
                        let _ = restart.send(()).await;
                    }
                });
                tracing::info!(
                    path = %path_for_log,
                    extensions = ?exts_for_log,
                    "custom restart watcher started"
                );
            }
        }
    }

    // Store watchers so they don't get dropped (they must live for process lifetime)
    std::mem::forget(watchers);

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

    // Open browser if requested
    if open_browser {
        let url = format!("http://127.0.0.1:{}", config.dev.public_port);
        match webbrowser::open(&url) {
            Ok(_) => tracing::info!("Opening browser at {}", url),
            Err(e) => tracing::warn!("Failed to open browser: {}", e),
        }
    }

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

        let max_attempts = 120;
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
