# Library API

flux-wasm-builder exposes a library API for programmatic use and testing.

## Module Structure

```rust
// src/lib.rs
pub mod build;
pub mod env_check;
pub mod init;

// Re-exports
pub use build::{
    BuildConfig, BuildError, run_build_loop, run_command_with_timeout,
    run_wasm_pack, run_wasm_pack_with_env, start_watcher, FileWatcher,
    serve_pkg_file, spa_fallback,
};

#[cfg(not(feature = "embed-assets"))]
pub use build::{inject_reload_script, ws_reload_handler, RELOAD_SCRIPT};

pub use env_check::EnvCheckError;
pub use init::{scaffold, InitError};
```

## Build Module

### BuildConfig

```rust
pub struct BuildConfig {
    pub frontend_crate_path: PathBuf,
    pub pkg_output_path: PathBuf,
    pub index_html_path: PathBuf,
    pub watch_debounce_ms: u64,
    pub reload_ws_path: String,
    pub port: u16,
    pub build_timeout_secs: u64,
}

impl BuildConfig {
    pub fn new<P: Into<PathBuf>>(frontend_crate_path: P) -> Self;
}
```

### BuildError

```rust
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("wasm-pack failed (exit code: {exit_code:?})")]
    WasmPackFailed { exit_code: Option<i32> },

    #[error("wasm-pack timed out after {secs}s")]
    Timeout { secs: u64 },

    #[error("failed to spawn wasm-pack: {source}")]
    SpawnFailed { source: std::io::Error },

    #[error("failed to wait on wasm-pack process: {source}")]
    WaitFailed { source: std::io::Error },

    #[error("failed to kill timed-out wasm-pack process: {source}")]
    KillFailed { source: std::io::Error },
}
```

### Functions

#### run_wasm_pack

```rust
pub async fn run_wasm_pack(config: &BuildConfig) -> Result<(), BuildError>
```

Run wasm-pack build with the given configuration.

#### run_wasm_pack_with_env

```rust
pub async fn run_wasm_pack_with_env(
    config: &BuildConfig,
    extra_env: &[(&str, &str)],
) -> Result<(), BuildError>
```

Run wasm-pack with additional environment variables. Useful for testing with a custom PATH.

#### run_command_with_timeout

```rust
pub async fn run_command_with_timeout(
    cmd: &mut tokio::process::Command,
    timeout_secs: u64,
) -> Result<std::process::ExitStatus, BuildError>
```

Run a command with a timeout. Used internally by `run_wasm_pack`.

#### run_build_loop

```rust
pub async fn run_build_loop<F, Fut>(
    build_rx: tokio::sync::mpsc::Receiver<()>,
    reload_tx: tokio::sync::broadcast::Sender<()>,
    build_fn: F,
)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), BuildError>> + Send,
```

Run the build coordinator loop. See [Build Subsystem](../architecture/build-subsystem.md) for details.

#### start_watcher

```rust
pub type FileWatcher = notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>;

pub fn start_watcher(
    watch_path: &Path,
    debounce_ms: u64,
    build_tx: tokio::sync::mpsc::Sender<()>,
) -> Result<FileWatcher, notify_debouncer_mini::notify::Error>
```

Start the file watcher. See [File Watching](../architecture/file-watching.md) for details.

#### serve_pkg_file

```rust
pub async fn serve_pkg_file(
    path: actix_web::web::Path<String>,
    config: actix_web::web::Data<BuildConfig>,
) -> impl actix_web::Responder
```

Serve a file from the pkg/ directory. See [Static Assets](../architecture/static-assets.md).

#### spa_fallback

```rust
pub async fn spa_fallback(
    config: actix_web::web::Data<BuildConfig>,
    dev_mode: actix_web::web::Data<DevMode>,
) -> impl actix_web::Responder
```

SPA fallback handler for unmatched routes.

## Reload Module (Dev Mode Only)

Available when `embed-assets` feature is not enabled.

### RELOAD_SCRIPT

```rust
pub const RELOAD_SCRIPT: &str = r#"<script>
(function() {
  function connect() {
    const ws = new WebSocket('ws://' + location.host + '/ws/reload');
    ws.onmessage = () => location.reload();
    ws.onclose = () => setTimeout(connect, 1000);
  }
  connect();
})();
</script>"#;
```

### inject_reload_script

```rust
pub fn inject_reload_script(html: &str) -> String
```

Inject the reload script before `</body>`.

### ws_reload_handler

```rust
pub async fn ws_reload_handler(
    req: actix_web::HttpRequest,
    body: actix_web::web::Payload,
    reload_tx: actix_web::web::Data<tokio::sync::broadcast::Sender<()>>,
) -> Result<actix_web::HttpResponse, actix_web::Error>
```

WebSocket handler for live reload.

## Init Module

### scaffold

```rust
pub fn scaffold(root: &Path, name: &str) -> Result<(), InitError>
```

Scaffold a new project. See [Project Scaffolding](../user-guide/scaffolding.md).

### InitError

```rust
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("directory '{0}' already exists")]
    AlreadyExists(PathBuf),

    #[error("environment check failed: {0}")]
    EnvCheck(#[from] EnvCheckError),

    #[error("failed to create directory '{path}': {source}")]
    CreateDir { path: PathBuf, source: std::io::Error },

    #[error("failed to write '{path}': {source}")]
    WriteFile { path: PathBuf, source: std::io::Error },
}
```

## Env Check Module

### EnvCheckError

```rust
#[derive(Debug, thiserror::Error)]
pub enum EnvCheckError {
    #[error("wasm-pack not found on PATH — install from https://rustwasm.github.io/wasm-pack/")]
    WasmPackNotFound,

    #[error("wasm-pack version {found} is below minimum required {required}")]
    WasmPackVersionTooOld { found: String, required: String },

    #[error("wasm32-unknown-unknown target not installed — run: rustup target add wasm32-unknown-unknown")]
    Wasm32TargetMissing,

    #[error("failed to invoke '{command}': {source}")]
    SpawnFailed { command: String, source: std::io::Error },

    #[error("failed to parse wasm-pack version output: {0}")]
    VersionParseFailed(String),
}
```

### Functions

```rust
pub fn wasm_pack_on_path() -> bool
pub fn wasm_pack_version_ok() -> Result<(), EnvCheckError>
pub fn wasm32_target_installed() -> Result<(), EnvCheckError>
```

## DevMode

```rust
#[derive(Clone, Copy, Debug)]
pub struct DevMode(pub bool);
```

Flag indicating whether the server is running in development mode.

## Usage Example

```rust
use flux_wasm_builder::{
    BuildConfig, run_wasm_pack, start_watcher,
    run_build_loop, serve_pkg_file, spa_fallback,
};
use actix_web::{App, HttpServer, web};
use tokio::sync::{mpsc, broadcast};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = BuildConfig::new("../frontend");
    
    // Initial build
    run_wasm_pack(&config).await.expect("build failed");
    
    // Set up channels
    let (build_tx, build_rx) = mpsc::channel(8);
    let (reload_tx, _) = broadcast::channel(16);
    
    // Start watcher
    let _watcher = start_watcher(
        &config.frontend_crate_path.join("src"),
        config.watch_debounce_ms,
        build_tx,
    ).expect("watcher failed");
    
    // Spawn build loop
    let config_clone = config.clone();
    tokio::spawn(async move {
        run_build_loop(build_rx, reload_tx, move || {
            let c = config_clone.clone();
            async move { run_wasm_pack(&c).await }
        }).await;
    });
    
    // Start server
    HttpServer::new(move || {
        App::new()
            .route("/pkg/{filename}", web::get().to(serve_pkg_file))
            .default_service(web::get().to(spa_fallback))
    })
    .bind(("127.0.0.1", config.port))?
    .run()
    .await
}
```
