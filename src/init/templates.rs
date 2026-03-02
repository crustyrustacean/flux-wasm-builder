//! Template contents for generated project files.

/// Workspace root Cargo.toml
pub fn workspace_cargo_toml(name: &str) -> String {
    format!(
        r#"[workspace]
resolver = "3"
members = ["backend", "frontend", "shared"]

[workspace.package]
name = "{name}"
version = "0.1.0"
edition = "2024"
"#
    )
}

/// Cargo config with alias for backend
pub fn cargo_config(name: &str) -> String {
    // Use the full package name for the alias
    format!(
        r#"[alias]
backend = "run -p {name}-backend"
start = "run -p {name}-backend"
"#
    )
}

/// .gitignore for the workspace
pub fn gitignore() -> &'static str {
    r#"target/
frontend/pkg/
"#
}

/// Backend Cargo.toml
pub fn backend_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}-backend"
version.workspace = true
edition.workspace = true

[dependencies]
actix-web = "4"
actix-ws = "0.3"
futures-util = "0.3"
tokio = {{ version = "1", features = ["full"] }}
serde_json = "1"
mime_guess = "2"
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
notify-debouncer-mini = "0.4"
thiserror = "1"
{name}-shared = {{ path = "../shared" }}

[features]
embed-assets = ["dep:include_dir"]

[dependencies.include_dir]
version = "0.7"
optional = true
"#
    )
}

/// Backend build.rs
pub fn backend_build_rs() -> &'static str {
    r#"fn main() {
    // Use the environment variable, not cfg!(), for feature detection in build.rs
    if std::env::var("CARGO_FEATURE_EMBED_ASSETS").is_ok() {
        let pkg = std::path::Path::new("../frontend/pkg");
        if !pkg.exists() || pkg.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            panic!(
                "\n\nembedding assets requires frontend/pkg/ to exist and be non-empty.\n\
                 Run this first:\n\n  \
                 wasm-pack build frontend/ --target web --release\n\n"
            );
        }
    }
}
"#
}

/// Backend main.rs
pub fn backend_main_rs() -> &'static str {
    r#"use actix_web::{web, App, HttpServer};
use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::{fmt, EnvFilter};

mod build_subsystem;
mod api;

use build_subsystem::build::{BuildConfig, run_wasm_pack};
use build_subsystem::build_coordinator::run_build_loop;
use build_subsystem::watcher::start_watcher;
use build_subsystem::reload::ws_reload_handler;
use build_subsystem::static_assets::{serve_pkg_file, spa_fallback};
use build_subsystem::DevMode;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    // When running via cargo alias from workspace root, CWD is workspace root
    let config = BuildConfig::new("frontend");
    let port = config.port;

    // Run initial frontend build before starting server
    tracing::info!("running initial frontend build");
    if let Err(e) = run_wasm_pack(&config).await {
        eprintln!("error: initial build failed: {e}");
        std::process::exit(1);
    }

    // Create channels for build coordination
    let (build_tx, build_rx) = mpsc::channel::<()>(8);
    let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);

    // Start file watcher for frontend source changes
    let _watcher = start_watcher(
        &config.frontend_crate_path.join("src"),
        config.watch_debounce_ms,
        build_tx,
    )?;
    tracing::info!("file watcher started");

    // Spawn build coordinator loop
    let reload_tx_clone = reload_tx.clone();
    let config_clone = config.clone();
    tokio::spawn(async move {
        run_build_loop(build_rx, reload_tx_clone, move || {
            let config = config_clone.clone();
            async move { run_wasm_pack(&config).await }
        }).await;
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(DevMode(true)))
            .app_data(web::Data::new(reload_tx.clone()))
            // API routes registered first
            .configure(api::configure)
            // Static asset routes
            .route("/pkg/{filename}", web::get().to(serve_pkg_file))
            // WebSocket reload endpoint
            .route("/ws/reload", web::get().to(ws_reload_handler))
            // SPA fallback - must be last
            .default_service(web::get().to(spa_fallback))
    })
    .bind(("127.0.0.1", port));

    match server {
        Ok(s) => {
            tracing::info!(port, "server listening");
            s.run().await?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("error: port {port} is already in use. Change the port in BuildConfig.");
            std::process::exit(1);
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

fn init_tracing() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_target(false)
        .compact()
        .init();
}
"#
}

/// Backend build_subsystem/mod.rs
pub fn backend_build_subsystem_mod() -> &'static str {
    r#"pub mod build;
pub mod build_coordinator;
pub mod watcher;
pub mod reload;
pub mod static_assets;

/// Flag indicating development mode.
/// When true, the reload script is injected into index.html.
#[derive(Clone, Copy, Debug)]
pub struct DevMode(pub bool);
"#
}

/// Backend build_subsystem/build.rs - wasm-pack invocation
pub fn backend_build_subsystem_build_rs() -> &'static str {
    r#"use std::path::PathBuf;
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
    /// Path to the frontend crate directory (relative to workspace root, e.g., "frontend")
    pub frontend_crate_path: PathBuf,
    /// Path to the pkg output directory (default: frontend_crate_path + "/pkg")
    pub pkg_output_path: PathBuf,
    /// Path to index.html (default: frontend_crate_path + "/index.html")
    pub index_html_path: PathBuf,
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
            Err(BuildError::Timeout { secs: config.build_timeout_secs })
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
    let mut child = cmd.spawn()
        .map_err(|e| BuildError::SpawnFailed { source: e })?;

    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(BuildError::WasmPackFailed { exit_code: status.code() }),
        Ok(Err(e)) => Err(BuildError::WaitFailed { source: e }),
        Err(_elapsed) => {
            let _ = child.kill().await; // best-effort; process is already considered failed
            Err(BuildError::Timeout { secs: timeout.as_secs() })
        }
    }
}
"#
}

/// Backend API module
pub fn backend_api_mod(name: &str) -> String {
    // Convert hyphens to underscores for valid Rust identifier
    let crate_name = name.replace('-', "_");
    format!(
        r#"use actix_web::{{web, HttpResponse, Responder}};
use serde_json::json;
use {crate_name}_shared::HelloResponse;

pub fn configure(cfg: &mut web::ServiceConfig) {{
    cfg.service(
        web::scope("/api")
            .route("/hello", web::get().to(hello))
            .route("/status", web::get().to(status))
    );
}}

async fn hello() -> impl Responder {{
    HttpResponse::Ok().json(HelloResponse {{
        message: "Hello from the backend!".to_string(),
    }})
}}

async fn status() -> impl Responder {{
    HttpResponse::Ok().json(json!({{
        "version": env!("CARGO_PKG_VERSION"),
        "status": "ok"
    }}))
}}
"#
    )
}

/// Frontend Cargo.toml
pub fn frontend_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}-frontend"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
yew = {{ version = "0.22.1", features = ["csr"] }}
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
wasm-logger = "0.2.0"
web-sys = {{ version = "0.3", features = [ "Window", "Document" ] }}
gloo-net = "0.5"
log = "0.4.29"
console_error_panic_hook = "0.1.7"
{name}-shared = {{ path = "../shared" }}
"#
    )
}

/// Frontend index.html
pub fn frontend_index_html(name: &str) -> String {
    let js_name = name.replace('-', "_");
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{name}</title>
</head>
<body>
    <script type="module">
        import init from './pkg/{js_name}_frontend.js';
        init();
    </script>
</body>
</html>
"#
    )
}

/// Frontend lib.rs
pub fn frontend_lib_rs(name: &str) -> String {
    // Convert hyphens to underscores for valid Rust identifier
    let crate_name = name.replace('-', "_");
    format!(
        r#"use yew::prelude::*;
use gloo_net::http::Request;
use {crate_name}_shared::HelloResponse;

#[component(App)]
fn app() -> Html {{
    let message = use_state(|| None::<String>);

    {{
        let message = message.clone();
        use_effect_with((), move |_| {{
            let message = message.clone();
            wasm_bindgen_futures::spawn_local(async move {{
                match Request::get("/api/hello")
                    .send()
                    .await
                {{
                    Ok(response) => {{
                        if let Ok(hello) = response.json::<HelloResponse>().await {{
                            message.set(Some(hello.message));
                        }}
                    }}
                    Err(e) => {{
                        web_sys::console::log_1(&format!("Error: {{:?}}", e).into());
                    }}
                }}
            }});
            || ()
        }});
    }}

    html! {{
        <div>
            <h1>{{ "Flux WASM Builder" }}</h1>
            {{
                if let Some(msg) = (*message).clone() {{
                    html! {{ <p>{{ msg }}</p> }}
                }} else {{
                    html! {{ <p>{{ "Loading..." }}</p> }}
                }}
            }}
        </div>
    }}
}}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {{
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}}
"#
    )
}

/// Shared Cargo.toml
pub fn shared_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}-shared"
version.workspace = true
edition.workspace = true

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
"#
    )
}

/// Shared lib.rs
pub fn shared_lib_rs() -> &'static str {
    r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
}
"#
}

/// Backend build_subsystem/watcher.rs - file watcher for .rs source changes
pub fn backend_build_subsystem_watcher_rs() -> &'static str {
    r#"use std::path::Path;
use std::time::Duration;

use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use tokio::sync::mpsc::Sender;

/// Type alias for the watcher type returned by [`start_watcher`].
pub type FileWatcher = notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>;

/// Start the file watcher.
///
/// # Important — Lifetime
/// The returned `Debouncer` **must be stored** for the lifetime of the process.
/// Dropping it silently stops all file watching with no error or warning.
/// Store it in a long-lived struct field, never in a local variable.
pub fn start_watcher(
    watch_path: &Path,
    debounce_ms: u64,
    build_tx: Sender<()>,
) -> Result<FileWatcher, notify_debouncer_mini::notify::Error> {
    let span = tracing::info_span!(
        "file_watcher",
        path = %watch_path.display(),
        debounce_ms
    );
    let _enter = span.enter();

    tracing::info!("starting file watcher");

    let mut debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    let rs_change = events.iter().any(|e| {
                        e.path.extension().map(|ext| ext == "rs").unwrap_or(false)
                    });

                    if rs_change {
                        tracing::debug!(event_count = events.len(), "Rust source change detected");
                        // Use blocking_send() because this callback runs on a non-async
                        // background thread (notify's thread pool), but the receiver is
                        // a tokio async channel.
                        if let Err(e) = build_tx.blocking_send(()) {
                            tracing::debug!(error = %e, "build channel closed — watcher callback exiting");
                        }
                    } else {
                        tracing::trace!("ignored non-.rs file event");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "file watcher error");
                }
            }
        },
    )?;

    debouncer.watcher().watch(watch_path, RecursiveMode::Recursive)?;
    tracing::info!("file watcher active");

    Ok(debouncer)
}
"#
}

/// Backend build_subsystem/build_coordinator.rs - build loop with coalescing
pub fn backend_build_subsystem_build_coordinator_rs() -> &'static str {
    r#"use std::future::Future;
use tokio::sync::{mpsc::Receiver, broadcast};

use super::build::BuildError;

/// Run the build loop, serializing rebuild requests.
///
/// The loop waits for signals on `build_rx`, executes `build_fn`,
/// and coalesces rapid triggers into at most two builds (one running, one pending).
/// After a successful build, sends a reload signal via `reload_tx`.
pub async fn run_build_loop<F, Fut>(
    mut build_rx: Receiver<()>,
    reload_tx: broadcast::Sender<()>,
    build_fn: F,
)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), BuildError>> + Send,
{
    let mut pending = false;
    
    loop {
        // If not pending, wait for a trigger
        if !pending {
            match build_rx.recv().await {
                None => {
                    tracing::debug!("build channel closed — coordinator exiting");
                    return;
                }
                Some(()) => {}
            }
        }
        
        // Reset pending flag - we're now processing this build
        pending = false;

        // Drain any queued messages and set pending flag for next iteration
        while build_rx.try_recv().is_ok() {
            pending = true;
        }
        if pending {
            tracing::debug!("coalesced rapid triggers — running one build with one queued");
        }

        // Execute build
        let span = tracing::info_span!("rebuild_cycle");
        let result = span.in_scope(|| build_fn()).await;

        match result {
            Ok(()) => {
                tracing::info!(parent: &span, "rebuild succeeded");
                // Send reload signal to connected browsers.
                // The `let _ =` is intentional: send returns Err when there
                // are no active subscribers, which is expected when no browser
                // tabs are open. This must not be treated as an error.
                let _ = reload_tx.send(());
            }
            Err(ref e) => {
                tracing::warn!(parent: &span, error = %e, "rebuild failed — previous assets still served");
            }
        }

        if pending {
            tracing::debug!("pending build trigger detected — starting next build immediately");
        }
    }
}
"#
}

/// Backend build_subsystem/static_assets.rs - asset serving handlers
pub fn backend_build_subsystem_static_assets_rs() -> &'static str {
    r#"use actix_web::{web, HttpResponse, Responder};

use super::build::BuildConfig;
use super::reload::inject_reload_script;
use super::DevMode;

/// Serve a file from the pkg/ directory.
///
/// # Security
/// Uses `file_name()` to extract only the final path component,
/// preventing path traversal attacks like `/pkg/../../etc/passwd`.
///
/// # MIME Types
/// Uses `mime_guess` for automatic MIME type detection.
/// `.wasm` files are served with `application/wasm`.
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();

    // Path traversal guard - extract only the final component
    // This prevents `/pkg/../secret.txt` from escaping the pkg directory
    let safe_name = match std::path::Path::new(&filename).file_name() {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!(filename = %filename, "path traversal attempt blocked");
            return HttpResponse::BadRequest().body("invalid filename");
        }
    };

    let file_path = config.pkg_output_path.join(&safe_name);
    tracing::debug!(path = %file_path.display(), "resolving asset");

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&safe_name).first_or_octet_stream();
            tracing::debug!(
                bytes = bytes.len(),
                mime = %mime,
                "serving asset"
            );
            HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(bytes)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(path = %file_path.display(), "asset not found");
            HttpResponse::NotFound().finish()
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                path = %file_path.display(),
                "failed to read asset"
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// SPA fallback handler - serves index.html for all unmatched routes.
///
/// This enables client-side routing in the Yew frontend.
/// Any route not matched by API or static asset handlers will receive index.html.
///
/// In development mode (`DevMode(true)`), the reload script is injected
/// into the HTML before serving to enable live reload.
pub async fn spa_fallback(
    config: web::Data<BuildConfig>,
    dev_mode: web::Data<DevMode>,
) -> impl Responder {
    let span = tracing::debug_span!("spa_fallback");
    let _enter = span.enter();

    match tokio::fs::read(&config.index_html_path).await {
        Ok(bytes) => {
            tracing::debug!(path = %config.index_html_path.display(), "serving index.html");
            
            let html = if dev_mode.0 {
                // Dev mode: inject reload script
                let html_str = String::from_utf8_lossy(&bytes);
                inject_reload_script(&html_str).into_bytes()
            } else {
                // Release mode: serve as-is
                bytes
            };
            
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(html)
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                path = %config.index_html_path.display(),
                "failed to read index.html"
            );
            HttpResponse::InternalServerError()
                .body("internal error: could not read index.html")
        }
    }
}
"#
}

/// Backend build_subsystem/reload.rs - WebSocket live reload
pub fn backend_build_subsystem_reload_rs() -> &'static str {
    r##"use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::stream::StreamExt;
use tokio::sync::broadcast;

/// JavaScript that establishes a WebSocket connection for live reload.
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

/// Inject the reload script into HTML content.
pub fn inject_reload_script(html: &str) -> String {
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + RELOAD_SCRIPT.len());
        result.push_str(&html[..pos]);
        result.push_str(RELOAD_SCRIPT);
        result.push_str(&html[pos..]);
        result
    } else {
        let mut result = String::with_capacity(html.len() + RELOAD_SCRIPT.len());
        result.push_str(html);
        result.push_str(RELOAD_SCRIPT);
        result
    }
}

/// WebSocket handler for live reload signaling.
pub async fn ws_reload_handler(
    req: HttpRequest,
    body: web::Payload,
    reload_tx: web::Data<broadcast::Sender<()>>,
) -> Result<HttpResponse, actix_web::Error> {
    let peer = req.peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "unknown".into());

    let span = tracing::debug_span!("ws_connection", peer = %peer);
    let _enter = span.enter();
    tracing::debug!("WebSocket connection established");

    let mut reload_rx = reload_tx.subscribe();
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                result = reload_rx.recv() => {
                    match result {
                        Ok(()) => {
                            tracing::debug!("sending reload signal to browser");
                            if session.text("reload").await.is_err() {
                                tracing::debug!("WebSocket send failed — client disconnected");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "WebSocket subscriber lagged — sending one reload");
                            let _ = session.text("reload").await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("broadcast channel closed — closing WebSocket");
                            break;
                        }
                    }
                }
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(Message::Close(reason))) => {
                            tracing::debug!(?reason, "WebSocket client closed connection");
                            let _ = session.close(reason).await;
                            break;
                        }
                        None | Some(Err(_)) => {
                            tracing::debug!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    Ok(response)
}
"##
}
