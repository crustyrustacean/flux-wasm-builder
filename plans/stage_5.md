# Stage 5 — Release Embedding Implementation Plan

**Status:** Planning  
**Prerequisites:** Stages 0-4 (All Complete)

---

## 1. Executive Summary

Stage 5 implements the `embed-assets` feature that enables single-binary deployment. When compiled with `--features embed-assets`, the frontend WebAssembly and JavaScript assets are embedded directly into the backend binary at compile time using `include_dir`. This eliminates the need for `wasm-pack` at runtime and produces a self-contained, zero-dependency binary.

### Key Deliverables

1. **`include_dir` Dependency** — Optional dependency with `embed-assets` feature
2. **Embedded Asset Static Variables** — `EMBEDDED_PKG` and `EMBEDDED_INDEX` 
3. **Feature-Gated Handlers** — Separate code paths for dev and release modes
4. **Feature-Gated WebSocket** — `/ws/reload` endpoint compiled out in release
5. **Feature-Gated Build Subsystem** — No watcher/coordinator in release mode
6. **Unit Tests** — Feature-gated tests for embedded assets
7. **Template Updates** — Generated projects include all release mode code

---

## 2. Current State Analysis

### What Exists

| Component | Location | Status |
|-----------|----------|--------|
| Build Pipeline | [`src/build/build.rs`](src/build/build.rs) | ✅ Complete |
| Build Coordinator | [`src/build/build_coordinator.rs`](src/build/build_coordinator.rs) | ✅ Complete |
| File Watcher | [`src/build/watcher.rs`](src/build/watcher.rs) | ✅ Complete |
| Reload Module | [`src/build/reload.rs`](src/build/reload.rs) | ✅ Complete |
| Static Assets | [`src/build/static_assets.rs`](src/build/static_assets.rs) | ⚠️ Dev mode only |
| Templates | [`src/init/templates.rs`](src/init/templates.rs) | ⚠️ Missing embed code |
| Tool Cargo.toml | [`Cargo.toml`](Cargo.toml) | ⚠️ Missing `include_dir` |

### What Is Missing

1. **`include_dir` dependency** — Not in tool's `Cargo.toml`
2. **Embedded asset statics** — No `EMBEDDED_PKG` or `EMBEDDED_INDEX`
3. **Feature-gated handlers** — `serve_pkg_file` and `spa_fallback` only work in dev mode
4. **Feature-gated WebSocket** — No `#[cfg]` gates on reload functionality
5. **Feature-gated build subsystem** — No conditional compilation for watcher/coordinator
6. **Template embedded asset code** — Generated projects lack release mode code
7. **Feature-gated tests** — No tests for embedded asset behavior

---

## 3. Implementation Steps

### Step 3.1: Add `include_dir` Dependency

**File:** [`Cargo.toml`](Cargo.toml)

Add `include_dir` as an optional dependency:

```toml
[dependencies]
# ... existing dependencies ...
include_dir = { version = "0.7", optional = true }

[features]
embed-assets = ["dep:include_dir"]
```

**Rationale:** The tool itself needs this for testing the embedded asset functionality. The templates already reference it for generated projects.

---

### Step 3.2: Create Embedded Asset Statics

**File:** [`src/build/static_assets.rs`](src/build/static_assets.rs)

Add after the module doc comment:

```rust
/// Embedded pkg/ directory for release mode.
/// Only compiled when the `embed-assets` feature is active.
#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir = 
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

/// Embedded index.html for release mode.
/// Only compiled when the `embed-assets` feature is active.
#[cfg(feature = "embed-assets")]
static EMBEDDED_INDEX: &[u8] = 
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/index.html"));
```

**Important Notes:**
- Uses `$CARGO_MANIFEST_DIR` to resolve paths relative to the crate root
- These statics are only compiled when `embed-assets` feature is active
- The paths assume a scaffolded project structure with `frontend/` as a sibling to `backend/`

---

### Step 3.3: Feature-Gate `serve_pkg_file`

**File:** [`src/build/static_assets.rs`](src/build/static_assets.rs)

Replace the current `serve_pkg_file` function with feature-gated versions:

```rust
/// Serve a file from the pkg/ directory.
///
/// # Security
/// Uses `file_name()` to extract only the final path component,
/// preventing path traversal attacks like `/pkg/../../etc/passwd`.
///
/// # MIME Types
/// Uses `mime_guess` for automatic MIME type detection.
/// `.wasm` files are served with `application/wasm`.
#[cfg(not(feature = "embed-assets"))]
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder {
    // Dev mode: read from filesystem
    let filename = path.into_inner();

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

/// Serve a file from the embedded pkg/ directory.
/// Release mode: assets are embedded at compile time.
#[cfg(feature = "embed-assets")]
pub async fn serve_pkg_file(
    path: web::Path<String>,
    _config: web::Data<BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();

    let safe_name = match std::path::Path::new(&filename).file_name() {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!(filename = %filename, "path traversal attempt blocked");
            return HttpResponse::BadRequest().body("invalid filename");
        }
    };

    // Search for file in embedded directory
    match EMBEDDED_PKG.get_file(&safe_name) {
        Some(file) => {
            let bytes = file.contents();
            let mime = mime_guess::from_path(&safe_name).first_or_octet_stream();
            tracing::debug!(
                bytes = bytes.len(),
                mime = %mime,
                "serving embedded asset"
            );
            HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(bytes.to_vec())
        }
        None => {
            tracing::debug!(filename = %safe_name.to_string_lossy(), "embedded asset not found");
            HttpResponse::NotFound().finish()
        }
    }
}
```

---

### Step 3.4: Feature-Gate `spa_fallback`

**File:** [`src/build/static_assets.rs`](src/build/static_assets.rs)

Replace the current `spa_fallback` function with feature-gated versions:

```rust
/// SPA fallback handler - serves index.html for all unmatched routes.
/// Dev mode: reads from filesystem and injects reload script.
#[cfg(not(feature = "embed-assets"))]
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
                let html_str = String::from_utf8_lossy(&bytes);
                inject_reload_script(&html_str).into_bytes()
            } else {
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

/// SPA fallback handler - serves embedded index.html.
/// Release mode: no reload script injection.
#[cfg(feature = "embed-assets")]
pub async fn spa_fallback(
    _config: web::Data<BuildConfig>,
    _dev_mode: web::Data<DevMode>,
) -> impl Responder {
    let span = tracing::debug_span!("spa_fallback");
    let _enter = span.enter();

    tracing::debug!("serving embedded index.html");
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(EMBEDDED_INDEX.to_vec())
}
```

---

### Step 3.5: Feature-Gate Reload Module

**File:** [`src/build/mod.rs`](src/build/mod.rs)

Update module declarations and exports:

```rust
// src/build/mod.rs

//! Build subsystem for invoking wasm-pack and serving static assets.

mod build;
mod build_coordinator;
mod static_assets;
mod watcher;

// Only compile reload module in dev mode
#[cfg(not(feature = "embed-assets"))]
mod reload;

pub use build::{BuildConfig, BuildError, run_wasm_pack, run_wasm_pack_with_env, run_command_with_timeout};
pub use build_coordinator::run_build_loop;
pub use static_assets::{serve_pkg_file, spa_fallback};
pub use watcher::{start_watcher, FileWatcher};

// Only export reload functionality in dev mode
#[cfg(not(feature = "embed-assets"))]
pub use reload::{inject_reload_script, ws_reload_handler, RELOAD_SCRIPT};

/// Flag indicating whether the server is running in development mode.
#[derive(Clone, Copy, Debug)]
pub struct DevMode(pub bool);
```

---

### Step 3.6: Feature-Gate lib.rs Exports

**File:** [`src/lib.rs`](src/lib.rs)

Update exports to conditionally include reload functionality:

```rust
// src/lib.rs

// module declarations
pub mod build;
pub mod env_check;
pub mod init;

// Re-exports for convenience
pub use build::{
    BuildConfig, BuildError, DevMode, run_build_loop, run_command_with_timeout, 
    run_wasm_pack, run_wasm_pack_with_env, start_watcher, FileWatcher,
    serve_pkg_file, spa_fallback,
};

// Only export reload functionality in dev mode
#[cfg(not(feature = "embed-assets"))]
pub use build::{inject_reload_script, ws_reload_handler, RELOAD_SCRIPT};

pub use env_check::EnvCheckError;
pub use init::{scaffold, InitError};
```

---

### Step 3.7: Add Feature-Gated Unit Tests

**File:** [`src/build/static_assets.rs`](src/build/static_assets.rs)

Add tests for embedded assets:

```rust
#[cfg(all(test, feature = "embed-assets"))]
mod embed_tests {
    use super::*;

    #[test]
    fn embedded_pkg_is_non_empty() {
        assert!(EMBEDDED_PKG.entries().count() > 0);
    }

    #[test]
    fn embedded_pkg_contains_wasm_file() {
        let has_wasm = EMBEDDED_PKG.entries().any(|e| {
            e.path().extension().map(|x| x == "wasm").unwrap_or(false)
        });
        assert!(has_wasm, "no .wasm file in embedded pkg/");
    }

    #[test]
    fn embedded_index_is_non_empty() {
        assert!(!EMBEDDED_INDEX.is_empty());
    }

    #[test]
    fn embedded_index_does_not_contain_reload_script() {
        let html = std::str::from_utf8(EMBEDDED_INDEX).unwrap();
        assert!(!html.contains("/ws/reload"));
    }
}
```

**Note:** These tests require:
1. The `embed-assets` feature to be active
2. A `frontend/pkg/` directory with built assets
3. A `frontend/index.html` file

---

### Step 3.8: Update Templates

**File:** [`src/init/templates.rs`](src/init/templates.rs)

#### 3.8.1: Update `backend_cargo_toml`

Already correct - includes `include_dir` as optional dependency with `embed-assets` feature.

#### 3.8.2: Update `backend_build_rs`

Already correct - uses `CARGO_FEATURE_EMBED_ASSETS` environment variable.

#### 3.8.3: Update `backend_main_rs`

Add feature gates for dev-mode only code:

```rust
pub fn backend_main_rs() -> &'static str {
    r#"use actix_web::{web, App, HttpServer};
use tracing_subscriber::{fmt, EnvFilter};

mod build_subsystem;
mod api;

use build_subsystem::build::{BuildConfig, run_wasm_pack};
use build_subsystem::static_assets::{serve_pkg_file, spa_fallback};
use build_subsystem::DevMode;

// Dev-mode only imports
#[cfg(not(feature = "embed-assets"))]
use build_subsystem::build_coordinator::run_build_loop;
#[cfg(not(feature = "embed-assets"))]
use build_subsystem::watcher::start_watcher;
#[cfg(not(feature = "embed-assets"))]
use build_subsystem::reload::ws_reload_handler;
#[cfg(not(feature = "embed-assets"))]
use tokio::sync::{broadcast, mpsc};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let config = BuildConfig::new("frontend");
    let port = config.port;

    // Dev mode: run initial frontend build and start watcher
    #[cfg(not(feature = "embed-assets"))]
    {
        tracing::info!("running initial frontend build");
        if let Err(e) = run_wasm_pack(&config).await {
            eprintln!("error: initial build failed: {e}");
            std::process::exit(1);
        }

        let (build_tx, build_rx) = mpsc::channel::<()>(8);
        let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);

        let _watcher = start_watcher(
            &config.frontend_crate_path.join("src"),
            config.watch_debounce_ms,
            build_tx,
        )?;
        tracing::info!("file watcher started");

        let reload_tx_clone = reload_tx.clone();
        let config_clone = config.clone();
        tokio::spawn(async move {
            run_build_loop(build_rx, reload_tx_clone, move || {
                let config = config_clone.clone();
                async move { run_wasm_pack(&config).await }
            }).await;
        });

        run_server(config, port, Some(reload_tx)).await?;
    }

    // Release mode: just start the server with embedded assets
    #[cfg(feature = "embed-assets")]
    {
        run_server(config, port, None).await?;
    }

    Ok(())
}

async fn run_server(
    config: BuildConfig,
    port: u16,
    #[cfg(not(feature = "embed-assets"))] reload_tx: Option<broadcast::Sender<()>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(DevMode(cfg!(not(feature = "embed-assets")))))
            .configure(api::configure)
            .route("/pkg/{filename}", web::get().to(serve_pkg_file));

        // Dev-mode only: WebSocket reload endpoint
        #[cfg(not(feature = "embed-assets"))]
        {
            app = app.route("/ws/reload", web::get().to(ws_reload_handler));
            if let Some(tx) = reload_tx.as_ref() {
                app = app.app_data(web::Data::new(tx.clone()));
            }
        }

        app.default_service(web::get().to(spa_fallback))
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
```

#### 3.8.4: Update `backend_build_subsystem_static_assets_rs`

Add embedded asset support to the template:

```rust
pub fn backend_build_subsystem_static_assets_rs() -> &'static str {
    r##"use actix_web::{web, HttpResponse, Responder};

use super::build::BuildConfig;
use super::DevMode;

// Dev-mode only import
#[cfg(not(feature = "embed-assets"))]
use super::reload::inject_reload_script;

// Embedded assets for release mode
#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir = 
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

#[cfg(feature = "embed-assets")]
static EMBEDDED_INDEX: &[u8] = 
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/index.html"));

/// Serve a file from the pkg/ directory.
/// Dev mode: reads from filesystem.
#[cfg(not(feature = "embed-assets"))]
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();

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
            HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(bytes)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::NotFound().finish()
        }
        Err(e) => {
            tracing::error!(error = %e, path = %file_path.display(), "failed to read asset");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Serve a file from the embedded pkg/ directory.
/// Release mode: assets are embedded at compile time.
#[cfg(feature = "embed-assets")]
pub async fn serve_pkg_file(
    path: web::Path<String>,
    _config: web::Data<BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();

    let safe_name = match std::path::Path::new(&filename).file_name() {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!(filename = %filename, "path traversal attempt blocked");
            return HttpResponse::BadRequest().body("invalid filename");
        }
    };

    match EMBEDDED_PKG.get_file(&safe_name) {
        Some(file) => {
            let bytes = file.contents();
            let mime = mime_guess::from_path(&safe_name).first_or_octet_stream();
            HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(bytes.to_vec())
        }
        None => HttpResponse::NotFound().finish()
    }
}

/// SPA fallback - serves index.html.
/// Dev mode: reads from filesystem and injects reload script.
#[cfg(not(feature = "embed-assets"))]
pub async fn spa_fallback(
    config: web::Data<BuildConfig>,
    dev_mode: web::Data<DevMode>,
) -> impl Responder {
    match tokio::fs::read(&config.index_html_path).await {
        Ok(bytes) => {
            let html = if dev_mode.0 {
                let html_str = String::from_utf8_lossy(&bytes);
                inject_reload_script(&html_str).into_bytes()
            } else {
                bytes
            };
            
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(html)
        }
        Err(e) => {
            tracing::error!(error = %e, path = %config.index_html_path.display(), "failed to read index.html");
            HttpResponse::InternalServerError()
                .body("internal error: could not read index.html")
        }
    }
}

/// SPA fallback - serves embedded index.html.
/// Release mode: no reload script.
#[cfg(feature = "embed-assets")]
pub async fn spa_fallback(
    _config: web::Data<BuildConfig>,
    _dev_mode: web::Data<DevMode>,
) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(EMBEDDED_INDEX.to_vec())
}
"##
}
```

#### 3.8.5: Update `backend_build_subsystem_mod`

Feature-gate the reload module:

```rust
pub fn backend_build_subsystem_mod() -> &'static str {
    r#"pub mod build;
pub mod build_coordinator;
pub mod watcher;
pub mod static_assets;

// Dev-mode only module
#[cfg(not(feature = "embed-assets"))]
pub mod reload;

pub use build::{BuildConfig, BuildError, run_wasm_pack};
pub use build_coordinator::run_build_loop;
pub use static_assets::{serve_pkg_file, spa_fallback};
pub use watcher::{start_watcher, FileWatcher};

#[cfg(not(feature = "embed-assets"))]
pub use reload::{inject_reload_script, ws_reload_handler, RELOAD_SCRIPT};

/// Flag indicating development mode.
#[derive(Clone, Copy, Debug)]
pub struct DevMode(pub bool);
"#
}
```

---

## 4. Test Plan

### 4.1 Unit Tests (Tool)

| Test | Location | Feature | Description |
|------|----------|---------|-------------|
| `embedded_pkg_is_non_empty` | `static_assets.rs` | `embed-assets` | Verifies embedded pkg has files |
| `embedded_pkg_contains_wasm_file` | `static_assets.rs` | `embed-assets` | Verifies .wasm file exists |
| `embedded_index_is_non_empty` | `static_assets.rs` | `embed-assets` | Verifies index.html has content |
| `embedded_index_does_not_contain_reload_script` | `static_assets.rs` | `embed-assets` | Verifies no /ws/reload in HTML |

**Running tests:**
```bash
# Standard tests (dev mode)
cargo test

# Embedded asset tests (requires frontend/pkg to exist)
wasm-pack build frontend/ --target web --dev
cargo test --features embed-assets
```

### 4.2 Integration Tests

| Test | File | Description |
|------|------|-------------|
| `build_rs_uses_env_var_not_cfg` | `tests/init.rs` | Verifies `CARGO_FEATURE_EMBED_ASSETS` usage |
| `backend_cargo_toml_has_embed_assets_feature` | `tests/init.rs` | Verifies feature definition |
| `embed_assets_requires_pkg_dir` | NEW | Verifies build.rs panic on missing pkg/ |

### 4.3 Manual Verification

1. **Build without pkg/ fails:**
   ```bash
   cargo build --features embed-assets
   # Should panic with "frontend/pkg/ does not exist"
   ```

2. **Build with pkg/ succeeds:**
   ```bash
   wasm-pack build frontend/ --target web --release
   cargo build --release --features embed-assets
   # Should succeed
   ```

3. **Release binary serves assets:**
   ```bash
   # Delete pkg/ from disk
   rm -rf frontend/pkg/
   # Run the binary
   ./target/release/flux-wasm-builder
   # GET /pkg/app_bg.wasm should return 200
   # GET /ws/reload should return 404
   ```

4. **No reload script in release:**
   ```bash
   curl http://localhost:8080/
   # Should not contain /ws/reload
   ```

---

## 5. Acceptance Criteria

| Criterion | Verification |
|-----------|--------------|
| `cargo build --features embed-assets` without `frontend/pkg/` fails with panic | Manual test |
| `backend/build.rs` uses `CARGO_FEATURE_EMBED_ASSETS` env var, not `cfg!()` | Unit test |
| After `wasm-pack build`, `cargo build --features embed-assets` succeeds | Manual test |
| Release binary serves `.wasm` with `Content-Type: application/wasm` | Integration test |
| `GET /ws/reload` on release binary returns 404 | Manual test |
| `index.html` served by release binary has no reload script | Unit test |
| All existing tests continue to pass | `cargo test` |

---

## 6. Implementation Order

1. **Add `include_dir` dependency** to `Cargo.toml`
2. **Update `src/build/static_assets.rs`** with embedded assets and feature-gated handlers
3. **Update `src/build/mod.rs`** with feature-gated module exports
4. **Update `src/lib.rs`** with feature-gated re-exports
5. **Add feature-gated unit tests** to `static_assets.rs`
6. **Update templates** in `src/init/templates.rs`:
   - `backend_main_rs()`
   - `backend_build_subsystem_mod()`
   - `backend_build_subsystem_static_assets_rs()`
7. **Add integration test** for build.rs behavior
8. **Run full test suite** and verify all acceptance criteria

---

## 7. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| `include_dir` path resolution differs across platforms | Use `$CARGO_MANIFEST_DIR` for portable paths |
| Embedded assets increase binary size significantly | Expected behavior; document typical sizes |
| Tests require built frontend assets | Use `#[cfg(feature = "embed-assets")]` to skip when unavailable |
| Template code diverges from tool code | Keep implementations identical; copy from tool to template |

---

## 8. Post-Implementation Verification

After implementation, verify:

1. **Dev mode unchanged:**
   ```bash
   cargo run
   # Should build frontend, start watcher, serve with reload
   ```

2. **Release mode works:**
   ```bash
   wasm-pack build frontend/ --target web --release
   cargo build --release --features embed-assets
   ./target/release/my-app-backend
   # Should serve embedded assets, no /ws/reload
   ```

3. **Generated projects work:**
   ```bash
   cargo run -- init test-project
   cd test-project
   cargo run  # Dev mode
   wasm-pack build frontend/ --target web --release
   cargo build --release --features embed-assets  # Release mode
   ```

---

*Plan created for Stage 5 — Release Embedding*
