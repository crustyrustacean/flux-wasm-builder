# Stage 2 Implementation Plan: Dev Server

## Overview

Stage 2 adds static file serving for `pkg/` contents and a catch-all SPA fallback. This enables the frontend and backend to communicate over the same origin with no CORS configuration.

## Current Status

### Completed Stages
- **Stage 0 (Project Scaffolding)**: Fully implemented - `init` command creates the three-crate workspace
- **Stage 1 (Static Build Pipeline)**: Fully implemented - `run_wasm_pack` function invokes wasm-pack with timeout handling

### Current Codebase Structure
```
src/
├── lib.rs                    # Module declarations and re-exports
├── bin/
│   └── main.rs               # CLI entry point with clap
├── env_check.rs              # Environment verification functions
├── build/
│   ├── mod.rs                # Build module exports
│   └── build.rs              # wasm-pack invocation and BuildConfig
└── init/
    ├── mod.rs                # Scaffold logic
    └── templates.rs          # Generated file templates
```

## Stage 2 Requirements

### From Spec Section 8 - Stage 2

1. **Static Asset Handlers** (`static_assets.rs`):
   - `serve_pkg_file` - Serves files from `pkg/` directory with MIME type detection
   - `spa_fallback` - Serves `index.html` for unmatched routes
   - **Path traversal guard** - Must prevent `../` attacks

2. **Route Registration Order** (critical):
   - API routes first (e.g., `/api/...`)
   - `/pkg/{filename}` for static assets
   - `spa_fallback` as `default_service` (must be last)

3. **MIME Type Handling**:
   - `.wasm` files must be served with `Content-Type: application/wasm`
   - Other files use `mime_guess` for type detection

### Acceptance Criteria
- [ ] `GET /pkg/app_bg.wasm` returns 200 with `Content-Type: application/wasm`
- [ ] `GET /pkg/nonexistent.js` returns 404
- [ ] Path traversal request returns 400 or 404, never 200
- [ ] `GET /any/unknown/path` returns `index.html` with status 200
- [ ] `GET /api/hello` returns JSON, not `index.html`
- [ ] Yew app communicates with `/api/hello` with no CORS errors

---

## Implementation Plan

### Step 1: Create `src/build/static_assets.rs`

Create the static asset serving module in the tool's source:

```rust
// src/build/static_assets.rs

use actix_web::{web, HttpResponse, Responder};
use std::path::PathBuf;

/// Serve a file from the pkg/ directory.
/// 
/// # Security
/// Uses `file_name()` to extract only the final path component,
/// preventing path traversal attacks like `/pkg/../../etc/passwd`.
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<crate::build::BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();
    
    // Path traversal guard - extract only the final component
    let safe_name = match std::path::Path::new(&filename).file_name() {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!("path traversal attempt blocked");
            return HttpResponse::BadRequest().body("invalid filename");
        }
    };
    
    let file_path = config.pkg_output_path.join(&safe_name);
    
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

/// SPA fallback handler - serves index.html for all unmatched routes.
pub async fn spa_fallback(
    config: web::Data<crate::build::BuildConfig>,
) -> impl Responder {
    match tokio::fs::read(&config.index_html_path).await {
        Ok(bytes) => {
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(bytes)
        }
        Err(e) => {
            tracing::error!(error = %e, path = %config.index_html_path.display(), "failed to read index.html");
            HttpResponse::InternalServerError()
                .body("internal error: could not read index.html")
        }
    }
}
```

### Step 2: Update `src/build/mod.rs`

Export the new static_assets module:

```rust
// src/build/mod.rs

mod build;
mod static_assets;  // Add this

pub use build::{BuildConfig, BuildError, run_wasm_pack, run_wasm_pack_with_env, run_command_with_timeout};
pub use static_assets::{serve_pkg_file, spa_fallback};  // Add this
```

### Step 3: Add Unit Tests to `static_assets.rs`

Per the spec, add the following tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, web};
    use tempfile::tempdir;
    use crate::build::BuildConfig;

    #[actix_web::test]
    async fn serves_js_file_from_pkg_directory() {
        let pkg_dir = tempdir().unwrap();
        std::fs::write(pkg_dir.path().join("app.js"), b"console.log('hi')").unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        let req = test::TestRequest::get().uri("/pkg/app.js").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 200);
    }

    #[actix_web::test]
    async fn serves_wasm_with_application_wasm_content_type() {
        let pkg_dir = tempdir().unwrap();
        std::fs::write(pkg_dir.path().join("app_bg.wasm"), b"\0asm").unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        let req = test::TestRequest::get().uri("/pkg/app_bg.wasm").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/wasm"), "unexpected content-type: {ct}");
    }

    #[actix_web::test]
    async fn returns_404_for_missing_pkg_file() {
        let pkg_dir = tempdir().unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        let req = test::TestRequest::get().uri("/pkg/nonexistent.js").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 404);
    }

    #[actix_web::test]
    async fn path_traversal_attempt_is_rejected() {
        let pkg_dir = tempdir().unwrap();
        let secret = pkg_dir.path().parent().unwrap().join("secret.txt");
        std::fs::write(&secret, b"secret content").unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        // URL-encoded traversal: /pkg/..%2Fsecret.txt
        let req = test::TestRequest::get()
            .uri("/pkg/..%2Fsecret.txt")
            .to_request();
        let status = test::call_service(&app, req).await.status();
        assert_ne!(status.as_u16(), 200, "path traversal must not return 200");
    }

    #[actix_web::test]
    async fn spa_fallback_serves_index_html() {
        let dir = tempdir().unwrap();
        let index = dir.path().join("index.html");
        std::fs::write(&index, b"<html></html>").unwrap();
        let config = BuildConfig {
            index_html_path: index,
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .default_service(web::get().to(spa_fallback))
        ).await;
        let req = test::TestRequest::get().uri("/any/unknown/path").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("text/html"), "unexpected content-type: {ct}");
    }

    #[actix_web::test]
    async fn api_route_takes_precedence_over_spa_fallback() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), b"<html></html>").unwrap();
        let config = BuildConfig {
            index_html_path: dir.path().join("index.html"),
            ..BuildConfig::new("/unused")
        };
        async fn hello() -> impl actix_web::Responder {
            actix_web::HttpResponse::Ok().json(serde_json::json!({"message": "hi"}))
        }
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/api/hello", web::get().to(hello))
                .default_service(web::get().to(spa_fallback))
        ).await;
        let req = test::TestRequest::get().uri("/api/hello").to_request();
        let resp = test::call_service(&app, req).await;
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(!ct.contains("text/html"), "API route returned index.html instead of JSON");
    }
}
```

### Step 4: Update `templates.rs` for Generated Projects

Add a template for `backend/src/build_subsystem/static_assets.rs`:

```rust
/// Backend build_subsystem/static_assets.rs - asset serving handlers
pub fn backend_build_subsystem_static_assets_rs() -> &'static str {
    r#"use actix_web::{web, HttpResponse, Responder};
use std::path::PathBuf;

use super::build::BuildConfig;

/// Serve a file from the pkg/ directory.
/// 
/// # Security
/// Uses `file_name()` to extract only the final path component,
/// preventing path traversal attacks like `/pkg/../../etc/passwd`.
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();
    
    // Path traversal guard - extract only the final component
    let safe_name = match std::path::Path::new(&filename).file_name() {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!("path traversal attempt blocked");
            return HttpResponse::BadRequest().body("invalid filename");
        }
    };
    
    let file_path = config.pkg_output_path.join(&safe_name);
    
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

/// SPA fallback handler - serves index.html for all unmatched routes.
pub async fn spa_fallback(
    config: web::Data<BuildConfig>,
) -> impl Responder {
    match tokio::fs::read(&config.index_html_path).await {
        Ok(bytes) => {
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(bytes)
        }
        Err(e) => {
            tracing::error!(error = %e, path = %config.index_html_path.display(), "failed to read index.html");
            HttpResponse::InternalServerError()
                .body("internal error: could not read index.html")
        }
    }
}
"#
}
```

### Step 5: Update `backend_main_rs` Template

Modify the main.rs template to wire up static asset routes:

```rust
pub fn backend_main_rs() -> &'static str {
    r#"use actix_web::{web, App, HttpServer};
use tracing_subscriber::{fmt, EnvFilter};

mod build_subsystem;
mod api;

use build_subsystem::build::{BuildConfig, run_wasm_pack};
use build_subsystem::static_assets::{serve_pkg_file, spa_fallback};

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

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            // API routes registered first
            .configure(api::configure)
            // Static asset routes
            .route("/pkg/{filename}", web::get().to(serve_pkg_file))
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
```

### Step 6: Update `backend_build_subsystem_mod` Template

Update the module declaration:

```rust
pub fn backend_build_subsystem_mod() -> &'static str {
    r#"pub mod build;
pub mod build_coordinator;
pub mod watcher;
pub mod reload;
pub mod static_assets;

pub use build::{BuildConfig, BuildError, run_wasm_pack};
pub use static_assets::{serve_pkg_file, spa_fallback};
"#
}
```

### Step 7: Update `src/init/mod.rs`

Ensure the scaffold function writes the `static_assets.rs` file:

```rust
// In the write_files function, add:
std::fs::write(
    project_root.join("backend/src/build_subsystem/static_assets.rs"),
    templates::backend_build_subsystem_static_assets_rs(),
)?;
```

### Step 8: Add Integration Tests

Create `tests/serve.rs`:

```rust
// tests/serve.rs
//
// Integration tests for static asset serving.

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

#[test]
fn scaffolded_backend_has_static_assets_module() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let static_assets_path = dir.path().join("test-app/backend/src/build_subsystem/static_assets.rs");
    assert!(static_assets_path.exists());

    let contents = std::fs::read_to_string(&static_assets_path).unwrap();
    assert!(contents.contains("serve_pkg_file"));
    assert!(contents.contains("spa_fallback"));
    assert!(contents.contains("file_name")); // Path traversal guard
}

#[test]
fn scaffolded_backend_main_wires_static_routes() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let main_rs_path = dir.path().join("test-app/backend/src/main.rs");
    let contents = std::fs::read_to_string(&main_rs_path).unwrap();

    // Check route registration order
    let api_pos = contents.find("/api").expect("API routes should be registered");
    let pkg_pos = contents.find("/pkg/{filename}").expect("pkg route should be registered");
    let default_pos = contents.find("default_service").expect("default_service should be registered");

    assert!(api_pos < pkg_pos, "API routes must be registered before pkg routes");
    assert!(pkg_pos < default_pos, "pkg routes must be registered before default_service");
}

#[test]
fn scaffolded_backend_main_imports_static_assets() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let main_rs_path = dir.path().join("test-app/backend/src/main.rs");
    let contents = std::fs::read_to_string(&main_rs_path).unwrap();

    assert!(contents.contains("use build_subsystem::static_assets"));
    assert!(contents.contains("serve_pkg_file"));
    assert!(contents.contains("spa_fallback"));
}
```

---

## Dependencies

The following dependencies are already in `Cargo.toml`:
- `actix-web` - HTTP server
- `tokio` - Async runtime
- `mime_guess` - MIME type detection
- `tracing` - Logging

No new dependencies are required for Stage 2.

---

## Testing Strategy

### Unit Tests (in `static_assets.rs`)
1. `serves_js_file_from_pkg_directory` - Basic file serving
2. `serves_wasm_with_application_wasm_content_type` - WASM MIME type
3. `returns_404_for_missing_pkg_file` - Missing file handling
4. `path_traversal_attempt_is_rejected` - Security test
5. `spa_fallback_serves_index_html` - SPA fallback
6. `api_route_takes_precedence_over_spa_fallback` - Route ordering

### Integration Tests (in `tests/serve.rs`)
1. `scaffolded_backend_has_static_assets_module` - File generation
2. `scaffolded_backend_main_wires_static_routes` - Route ordering
3. `scaffolded_backend_main_imports_static_assets` - Import verification

---

## Files to Modify/Create

| File | Action |
|------|--------|
| `src/build/static_assets.rs` | Create |
| `src/build/mod.rs` | Modify - add export |
| `src/init/templates.rs` | Modify - add template functions |
| `src/init/mod.rs` | Modify - write static_assets.rs |
| `tests/serve.rs` | Create |

---

## Verification Checklist

After implementation, verify:

- [ ] All unit tests pass: `cargo test`
- [ ] All integration tests pass: `cargo test --test serve`
- [ ] `cargo run` in a scaffolded project serves the frontend
- [ ] `GET /pkg/*.wasm` returns correct MIME type
- [ ] Path traversal is blocked
- [ ] SPA fallback works for unknown routes
- [ ] API routes take precedence over SPA fallback
