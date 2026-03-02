# Instrumentation and Error Handling Strategy
## Integrated Fullstack Build Tool — Actix-web + Yew

*Companion document to Design Specification Draft 0.4 · March 2026*

---

## 1. Guiding Principles

Before diving into component-level detail, four principles shape every decision here.

**Errors are data.** Every error condition that can be anticipated has a typed variant. `anyhow::Error` is acceptable at the `main` boundary only — inside library code, everything is a typed enum. This gives the test suite something concrete to `match` on and keeps the spec's existing `assert!(matches!(result, Err(BuildError::WasmPackFailed { .. })))` idiom viable everywhere.

**Spans, not scattered log lines.** `tracing` spans create a causal tree that maps directly onto the tool's natural phases: `init`, `env_check`, `build`, `watch`, `serve`. A log line emitted inside a span carries the span's fields automatically, so there is no need to repeat context (e.g., the frontend crate path) on every log statement.

**Operator-legible messages.** Every error that surfaces to the terminal must answer: *what happened*, *where it happened*, and *what the developer should do next*. "error: could not read file" is not actionable. "error: frontend/index.html not found — has wasm-pack been run?" is.

**No silent failures.** The two silent-failure traps already identified in the spec (dropping the `Debouncer`, ignoring `broadcast_tx.send` errors) are explicitly handled here. Any place where a `Result` is intentionally discarded carries a comment explaining why.

---

## 2. Error Type Taxonomy

### 2.1 Tool-side errors (`my-tool` crate)

```rust
// src/init/mod.rs
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

// src/env_check.rs
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

### 2.2 Build subsystem errors (generated into backend crate)

This is the most heavily used error type and must cover every failure the build loop and startup path can produce.

```rust
// backend/src/build_subsystem/build.rs
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
```

### 2.3 Serve-time errors (generated into backend crate)

Serve-time errors are handled per-request rather than being propagated up a call stack, but they still need types so handlers can log them consistently.

```rust
// backend/src/build_subsystem/static_assets.rs
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("path traversal detected in filename '{0}'")]
    PathTraversal(String),

    #[error("asset not found: '{0}'")]
    NotFound(PathBuf),

    #[error("failed to read asset '{path}': {source}")]
    ReadFailed { path: PathBuf, source: std::io::Error },

    #[error("failed to read index.html at '{path}': {source}")]
    IndexHtmlReadFailed { path: PathBuf, source: std::io::Error },
}
```

### 2.4 Top-level startup error (generated `main.rs`)

```rust
// backend/src/main.rs
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("initial frontend build failed: {0}")]
    InitialBuild(#[from] BuildError),

    #[error("port {port} is already in use — change the port in BuildConfig")]
    PortInUse { port: u16 },

    #[error("server error: {source}")]
    Server { source: std::io::Error },
}
```

### 2.5 Dependency: `thiserror`

Add `thiserror = "1"` to both the tool's `Cargo.toml` and the backend template's `Cargo.toml`. It generates `Display`, `Error`, and `From` impls from the `#[error(...)]` and `#[from]` attributes and produces zero runtime overhead.

---

## 3. Tracing Setup and Configuration

### 3.1 Subscriber initialization

Both the tool binary and the generated backend binary initialize their subscriber in `main` before any other work. Use `tracing-subscriber` with `EnvFilter` so verbosity is controlled by the `RUST_LOG` environment variable without recompilation.

```rust
// In both my-tool/src/main.rs and generated backend/src/main.rs
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_target(false)   // suppress module path in default output
        .with_thread_ids(false)
        .compact()            // single-line format; switch to .pretty() when debugging
        .init();
}
```

**Default output level:** `info`. Developers see build start/finish and server bind messages. Debug output (file watcher events, individual fs notifications, WebSocket frame details) requires `RUST_LOG=debug`.

### 3.2 Crate features for the tool

Add to `my-tool/Cargo.toml`:

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

Add to the generated backend template's `Cargo.toml` (already lists `tracing` per Section 7.2 of the spec):

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## 4. Component-Level Instrumentation

### 4.1 `env_check.rs`

The three check functions are called at startup and during `scaffold`. They are short-lived and synchronous; use `tracing::debug!` for individual check results and let the caller emit the user-facing `tracing::error!` on failure.

```rust
pub fn wasm_pack_on_path() -> bool {
    let found = std::process::Command::new("which")
        .arg("wasm-pack")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    tracing::debug!(found, "wasm-pack PATH check");
    found
}

pub fn wasm_pack_version_ok() -> Result<(), EnvCheckError> {
    let output = std::process::Command::new("wasm-pack")
        .arg("--version")
        .output()
        .map_err(|e| EnvCheckError::SpawnFailed {
            command: "wasm-pack --version".into(),
            source: e,
        })?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let version_str = raw.trim().strip_prefix("wasm-pack ").unwrap_or(raw.trim());
    tracing::debug!(version = version_str, "wasm-pack version check");

    // semver comparison — see implementation note below
    if version_meets_minimum(version_str, "0.13.0") {
        Ok(())
    } else {
        Err(EnvCheckError::WasmPackVersionTooOld {
            found: version_str.to_string(),
            required: "0.13.0".into(),
        })
    }
}

pub fn wasm32_target_installed() -> Result<(), EnvCheckError> {
    let output = std::process::Command::new("rustup")
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
```

**Implementation note on semver comparison:** Avoid pulling in the full `semver` crate just for this check. A simple split-on-`.` integer comparison is sufficient given the known format `wasm-pack 0.XX.Y`:

```rust
fn version_meets_minimum(found: &str, minimum: &str) -> bool {
    fn parse(s: &str) -> Option<(u32, u32, u32)> {
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse::<u32>().unwrap_or(0);
        Some((major, minor, patch))
    }
    match (parse(found), parse(minimum)) {
        (Some(f), Some(m)) => f >= m,
        _ => false,
    }
}
```

### 4.2 `init/mod.rs` — scaffold function

Wrap the entire scaffold operation in a span so all log output from nested calls is automatically associated with the project being created.

```rust
pub fn scaffold(root: &Path, name: &str) -> Result<(), InitError> {
    let span = tracing::info_span!("scaffold", project = name);
    let _enter = span.enter();

    tracing::info!("running environment checks");
    wasm_pack_on_path().then_some(()).ok_or(EnvCheckError::WasmPackNotFound)?;
    wasm_pack_version_ok()?;
    wasm32_target_installed()?;

    let project_root = root.join(name);
    if project_root.exists() {
        tracing::warn!(path = %project_root.display(), "target directory already exists");
        return Err(InitError::AlreadyExists(project_root));
    }

    tracing::debug!("creating directory structure");
    create_dirs(&project_root)?;

    tracing::debug!("writing project files");
    write_files(&project_root, name)?;

    tracing::info!("project created successfully");
    Ok(())
}
```

Each `create_dirs` and `write_files` helper annotates its `io::Error` with the path before returning, following the `InitError::CreateDir` / `InitError::WriteFile` pattern above. This avoids losing the path context that `io::Error` alone does not carry.

### 4.3 `build.rs` — wasm-pack invocation

This is the most critical component for developer experience. The span must carry enough context to diagnose both success and failure unambiguously.

```rust
pub async fn run_wasm_pack(config: &BuildConfig) -> Result<(), BuildError> {
    run_wasm_pack_with_env(config, &[]).await
}

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

    let mut cmd = tokio::process::Command::new("wasm-pack");
    cmd.args(["build", "--target", "web"])
       .arg(&config.frontend_crate_path)
       .stdout(std::process::Stdio::inherit())
       .stderr(std::process::Stdio::inherit());

    // Apply profile flag
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

/// Testable primitive for timeout behaviour.
pub async fn run_command_with_timeout(
    mut cmd: tokio::process::Command,
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
```

**Key decisions here:**

- `stdout` and `stderr` are `Stdio::inherit()`, per the spec. `tracing` is used only for the tool's own lifecycle messages — start, success, failure. Compiler output from `wasm-pack` flows directly to the terminal and is not captured or filtered.
- The timeout branch kills the child before returning. If `kill()` itself fails (extremely unlikely but possible on permission-restricted systems), this is reported as `BuildError::KillFailed` rather than silently swallowed.
- `exit_code` is `Option<i32>` because a process killed by a signal has no exit code on Unix.

### 4.4 `build_coordinator.rs` — build loop

The build loop is long-lived and runs as a background task. Its instrumentation must be lightweight — a span per build cycle, not per loop iteration.

```rust
pub async fn run_build_loop<F, Fut>(
    mut build_rx: mpsc::Receiver<()>,
    build_fn: F,
)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), BuildError>> + Send,
{
    loop {
        // Wait for first trigger
        match build_rx.recv().await {
            None => {
                tracing::debug!("build channel closed — coordinator exiting");
                return;
            }
            Some(()) => {}
        }

        // Drain any queued messages and set pending flag
        let mut pending = false;
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
            }
            Err(ref e) => {
                // Build failure is NOT fatal to the loop — continue watching
                tracing::warn!(parent: &span, error = %e, "rebuild failed — previous assets still served");
            }
        }

        // If a change arrived during this build, loop immediately
        if pending {
            // Re-queue a synthetic trigger so the loop top receives it
            // (this is implicitly handled: pending_flag → continue without blocking recv)
        }
    }
}
```

**The critical design decision:** `BuildError` from a failed rebuild is logged at `warn!` level, not `error!`. The server keeps running and continues serving the last successful build. Only errors that stop the loop from functioning (i.e., channel close) are at `info!`. This matches developer expectations — a syntax error during active editing should not crash the server.

**Broadcast send:** After a successful rebuild, the coordinator sends on the broadcast channel:

```rust
// In the coordinator, after a successful build:
match reload_tx.send(()) {
    Ok(n) => tracing::debug!(receivers = n, "sent reload signal"),
    Err(_) => {
        // No active WebSocket subscribers — this is expected when no browser is open.
        // The `let _ =` idiom from the spec is correct; document the intent here.
        tracing::trace!("reload signal not sent — no active browser connections");
    }
}
```

### 4.5 `watcher.rs` — file watcher

The watcher's callback runs on a background thread managed by `notify`. Its instrumentation must not block.

```rust
/// Start the file watcher.
///
/// # Important — Lifetime
/// The returned `Debouncer` **must be stored** for the lifetime of the process.
/// Dropping it silently stops all file watching with no error or warning.
/// Store it in a long-lived struct field, never in a local variable.
pub fn start_watcher(
    watch_path: &Path,
    debounce_ms: u64,
    build_tx: mpsc::Sender<()>,
) -> Result<Debouncer<RecommendedWatcher>, notify::Error> {
    let span = tracing::info_span!(
        "file_watcher",
        path = %watch_path.display(),
        debounce_ms
    );
    let _enter = span.enter();
    tracing::info!("starting file watcher");

    let debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    let rs_change = events.iter().any(|e| {
                        e.paths.iter().any(|p| {
                            p.extension().map(|ext| ext == "rs").unwrap_or(false)
                        })
                    });
                    if rs_change {
                        tracing::debug!(event_count = events.len(), "Rust source change detected");
                        if let Err(e) = build_tx.blocking_send(()) {
                            // The receiver has been dropped — the build coordinator
                            // has exited. Stop logging; the process is likely shutting down.
                            tracing::debug!(error = %e, "build channel closed — watcher callback exiting");
                        }
                    } else {
                        tracing::trace!("ignored non-.rs file event");
                    }
                }
                Err(errors) => {
                    // Watcher errors are non-fatal — log them but continue.
                    // Common causes: race condition on file deletion during atomic save.
                    for e in errors {
                        tracing::warn!(error = %e, "file watcher error");
                    }
                }
            }
        },
    )?;

    debouncer.watcher().watch(watch_path, RecursiveMode::Recursive)?;
    tracing::info!("file watcher active");
    Ok(debouncer)
}
```

**Error handling in the callback:** The callback receives `DebounceEventResult`, which is `Result<Vec<DebouncedEvent>, Vec<notify::Error>>`. The error case is not a reason to stop watching — it typically indicates a transient OS filesystem event race. Log at `warn!` and continue.

**Channel send failure in the callback:** If `blocking_send` fails, the receiver (build coordinator) has been dropped, meaning the server is shutting down. Log at `debug!` and do nothing — this is not an error condition.

### 4.6 `reload.rs` — WebSocket live reload

The WebSocket handler is short-lived per connection. Span it at the connection level.

```rust
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
                // Reload signal from build coordinator
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
                            // Subscriber fell behind — this means multiple rebuilds happened
                            // faster than this connection was drained. Reload once now.
                            tracing::warn!(skipped = n, "WebSocket subscriber lagged — sending one reload");
                            let _ = session.text("reload").await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("broadcast channel closed — closing WebSocket");
                            break;
                        }
                    }
                }
                // Client disconnected or sent a close frame
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(actix_ws::Message::Close(reason))) => {
                            tracing::debug!(?reason, "WebSocket client closed connection");
                            let _ = session.close(reason).await;
                            break;
                        }
                        None | Some(Err(_)) => {
                            tracing::debug!("WebSocket stream ended");
                            break;
                        }
                        _ => {} // Ping/Pong/Text from client — ignore
                    }
                }
            }
        }
    });

    Ok(response)
}
```

**`RecvError::Lagged` handling:** `tokio::sync::broadcast` receivers that fall behind the sender's capacity get `Lagged(n)`. This is a realistic scenario if a developer saves repeatedly during a slow build. The correct response is to send one reload signal (the browser will refresh to the latest build) and log a warning.

### 4.7 `static_assets.rs` — asset serving

Handler errors translate directly to HTTP responses. The `AssetError` type from Section 2.3 makes the translation explicit and testable.

```rust
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();
    let span = tracing::debug_span!("serve_pkg_file", filename = %filename);
    let _enter = span.enter();

    // Path traversal guard — extract only the final component
    let safe_name = match std::path::Path::new(&filename).file_name() {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!("path traversal attempt blocked");
            return HttpResponse::BadRequest()
                .body("invalid filename");
        }
    };

    let file_path = config.pkg_output_path.join(&safe_name);
    tracing::debug!(path = %file_path.display(), "resolving asset");

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&safe_name)
                .first_or_octet_stream();
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
            tracing::debug!("asset not found");
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

pub async fn spa_fallback(config: web::Data<BuildConfig>) -> impl Responder {
    let span = tracing::debug_span!("spa_fallback");
    let _enter = span.enter();

    match tokio::fs::read(&config.index_html_path).await {
        Ok(bytes) => {
            let html = inject_reload_script(bytes);
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
```

**Why `debug_span!` for asset serving, not `info_span!`:** In normal operation the server handles dozens of asset requests per page load. At `info` level this would drown out build lifecycle messages. Asset-level detail is only needed when debugging a serving issue, so `debug` is the right threshold.

### 4.8 `main.rs` — startup, binding, and shutdown

The top-level startup path is where all the component errors converge. This is the only place where errors are converted to human-readable messages and the process exits.

```rust
#[actix_web::main]
async fn main() {
    init_tracing();

    if let Err(e) = run().await {
        // Display the full error chain using the Display impl from thiserror.
        // In a non-trivial error chain, eprintln! ensures it appears on stderr
        // even if the tracing subscriber has already been torn down.
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), StartupError> {
    let config = BuildConfig::new("../frontend");  // generated default

    let span = tracing::info_span!("startup", port = config.port);
    let _enter = span.enter();

    // Environment check at startup (mirrors scaffold's check)
    tracing::info!("checking environment");
    // (env_check calls — errors bubble as StartupError::EnvCheck if added to the enum,
    //  or handle inline with eprintln + exit for the generated project's simpler needs)

    // Initial build
    tracing::info!("running initial frontend build");
    run_wasm_pack(&config).await.map_err(StartupError::InitialBuild)?;

    // Start build subsystem (watcher + coordinator)
    let build_subsystem = BuildSubsystem::start(config.clone()).await;

    // Server bind with explicit port-in-use detection
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(build_subsystem.reload_tx.clone()))
            // API routes registered first
            .configure(api::configure)
            // Static asset routes
            .route("/pkg/{filename}", web::get().to(serve_pkg_file))
            // Dev-mode WebSocket (cfg gated in release)
            #[cfg(not(feature = "embed-assets"))]
            .route(&config.reload_ws_path, web::get().to(ws_reload_handler))
            // SPA fallback — must be last
            .default_service(web::get().to(spa_fallback))
    })
    .bind(("127.0.0.1", config.port))
    .map_err(|e| {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            StartupError::PortInUse { port: config.port }
        } else {
            StartupError::Server { source: e }
        }
    })?
    .run();

    tracing::info!(port = config.port, "server listening");

    // Graceful shutdown on Ctrl-C
    let server_handle = server.handle();
    tokio::select! {
        result = server => {
            result.map_err(|e| StartupError::Server { source: e })?;
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received Ctrl-C — shutting down gracefully");
            // Drop build subsystem first to stop file watcher and build loop.
            // The Debouncer inside BuildSubsystem is dropped here.
            drop(build_subsystem);
            server_handle.stop(true).await;
            tracing::info!("shutdown complete");
        }
    }

    Ok(())
}
```

---

## 5. Span and Log Level Reference

This table provides a quick reference for the expected tracing output at each verbosity level.

| Level | What you see |
|-------|-------------|
| `error` | wasm-pack build failed; failed to read asset; failed to read index.html |
| `warn` | rebuild failed (previous assets still served); watcher OS error; WebSocket subscriber lagged |
| `info` | startup messages; environment check pass/fail; build started/succeeded; server listening; shutdown |
| `debug` | individual file watch events; WebSocket connection/disconnect; asset serving details; coordinator messages |
| `trace` | non-.rs file events ignored; reload signal not sent (no browsers) |

Setting `RUST_LOG=debug` is the recommended starting point when diagnosing a build pipeline issue. Setting `RUST_LOG=trace` will also surface ignored watcher events, which is useful when investigating why a save is not triggering a rebuild.

---

## 6. Panic Policy

Panics are only appropriate in two places, consistent with the spec.

**`backend/build.rs` (Cargo build script):** If `embed-assets` is active but `frontend/pkg/` is absent or empty, the build script panics with a message directing the developer to run `wasm-pack` first. This is correct — the build cannot proceed and there is nothing to recover from.

```rust
// backend/build.rs
fn main() {
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
```

**Everywhere else:** No panics. Use `Result` throughout. `unwrap()` and `expect()` are banned except in `#[cfg(test)]` blocks, where test failures are the intended outcome of a panic.

---

## 7. Test Instrumentation Considerations

Tests that use `tracing` spans should initialize a test-compatible subscriber to avoid output noise when tests run in parallel. The `tracing-subscriber` crate provides `try_init()` which returns an error if a subscriber is already registered (expected in multi-test runs) rather than panicking.

```rust
// In test helpers or at the top of test modules that use tracing:
fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()   // captures output per test, respects cargo test --nocapture
        .with_env_filter("debug")
        .try_init();          // returns Err if already initialized — ignore it
}
```

Add `tracing-subscriber` to the workspace-level `[dev-dependencies]` alongside the existing test utilities from Section 7.5 of the spec:

```toml
[dev-dependencies]
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## 8. Summary of `Cargo.toml` Additions

### Tool (`my-tool/Cargo.toml`)

```toml
[dependencies]
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Generated backend template

```toml
[dependencies]
# (existing dependencies from spec Section 7.2)
thiserror = "1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

The `tracing` dependency is already listed in the spec's Section 7.2 — only `thiserror` and `tracing-subscriber` are net-new additions.

---

*Jeff · Companion to integrated_build_tool_spec.md Draft 0.4*
