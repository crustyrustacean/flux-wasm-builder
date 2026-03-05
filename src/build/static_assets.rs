// src/build/static_assets.rs

//! Static asset serving handlers for the dev server.
//!
//! This module provides handlers for serving:
//! - Files from the `pkg/` directory (WASM, JS, and other build artifacts)
//! - SPA fallback to `index.html` for client-side routing
//!
//! In development mode, the SPA fallback injects a reload script
//! that establishes a WebSocket connection for live reload.
//!
//! In release mode (with `embed-assets` feature), assets are embedded
//! directly into the binary at compile time using `include_dir`.

use actix_web::{HttpResponse, Responder, web};

use super::DevMode;
use super::wasm_pack::BuildConfig;

// Dev-mode only import for reload script injection
#[cfg(not(feature = "embed-assets"))]
use super::reload::inject_reload_script;

// Embedded assets for release mode
#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

#[cfg(feature = "embed-assets")]
static EMBEDDED_INDEX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../frontend/index.html"
));

/// Serve a file from the pkg/ directory.
///
/// # Security
/// Uses `file_name()` to extract only the final path component,
/// preventing path traversal attacks like `/pkg/../../etc/passwd`.
///
/// # MIME Types
/// Uses `mime_guess` for automatic MIME type detection.
/// `.wasm` files are served with `application/wasm`.
///
/// # Dev Mode
/// Reads files from the filesystem at runtime.
#[cfg(not(feature = "embed-assets"))]
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

/// Serve a file from the embedded pkg/ directory.
///
/// # Security
/// Uses `file_name()` to extract only the final path component,
/// preventing path traversal attacks like `/pkg/../../etc/passwd`.
///
/// # MIME Types
/// Uses `mime_guess` for automatic MIME type detection.
/// `.wasm` files are served with `application/wasm`.
///
/// # Release Mode
/// Assets are embedded at compile time; no filesystem access.
#[cfg(feature = "embed-assets")]
pub async fn serve_pkg_file(
    path: web::Path<String>,
    _config: web::Data<BuildConfig>,
) -> impl Responder {
    let filename = path.into_inner();

    // Path traversal guard - extract only the final component
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

/// SPA fallback handler - serves index.html for all unmatched routes.
///
/// This enables client-side routing in the Yew frontend.
/// Any route not matched by API or static asset handlers will receive index.html.
///
/// In development mode (`DevMode(true)`), the reload script is injected
/// into the HTML before serving to enable live reload.
///
/// # Dev Mode
/// Reads index.html from the filesystem and injects reload script.
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
            HttpResponse::InternalServerError().body("internal error: could not read index.html")
        }
    }
}

/// SPA fallback handler - serves embedded index.html.
///
/// # Release Mode
/// Serves the index.html that was embedded at compile time.
/// No reload script is injected.
#[cfg(feature = "embed-assets")]
pub async fn spa_fallback(
    _config: web::Data<BuildConfig>,
    _dev_mode: web::Data<DevMode>,
) -> impl Responder {
    tracing::debug!("serving embedded index.html");

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(EMBEDDED_INDEX.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};
    use tempfile::tempdir;

    // Note: do not annotate the return type of App::new() chains.
    // Let type inference handle the complex associated types.

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
                .route("/pkg/{filename}", web::get().to(serve_pkg_file)),
        )
        .await;
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
                .route("/pkg/{filename}", web::get().to(serve_pkg_file)),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/pkg/app_bg.wasm")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            ct.contains("application/wasm"),
            "unexpected content-type: {ct}"
        );
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
                .route("/pkg/{filename}", web::get().to(serve_pkg_file)),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/pkg/nonexistent.js")
            .to_request();
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
                .route("/pkg/{filename}", web::get().to(serve_pkg_file)),
        )
        .await;
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
                .app_data(web::Data::new(DevMode(false)))
                .default_service(web::get().to(spa_fallback)),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/any/unknown/path")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
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
                .app_data(web::Data::new(DevMode(false)))
                .route("/api/hello", web::get().to(hello))
                .default_service(web::get().to(spa_fallback)),
        )
        .await;
        let req = test::TestRequest::get().uri("/api/hello").to_request();
        let resp = test::call_service(&app, req).await;
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            !ct.contains("text/html"),
            "API route returned index.html instead of JSON"
        );
    }

    #[actix_web::test]
    async fn index_html_contains_reload_script_when_dev_mode_true() {
        let dir = tempdir().unwrap();
        let index = dir.path().join("index.html");
        std::fs::write(&index, b"<html><body></body></html>").unwrap();
        let config = BuildConfig {
            index_html_path: index,
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(DevMode(true)))
                .default_service(web::get().to(spa_fallback)),
        )
        .await;
        let body = test::read_body(
            test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await,
        )
        .await;
        assert!(
            std::str::from_utf8(&body).unwrap().contains("/ws/reload"),
            "index.html should contain reload script in dev mode"
        );
    }

    #[actix_web::test]
    async fn index_html_omits_reload_script_when_dev_mode_false() {
        let dir = tempdir().unwrap();
        let index = dir.path().join("index.html");
        std::fs::write(&index, b"<html><body></body></html>").unwrap();
        let config = BuildConfig {
            index_html_path: index,
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(DevMode(false)))
                .default_service(web::get().to(spa_fallback)),
        )
        .await;
        let body = test::read_body(
            test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await,
        )
        .await;
        assert!(
            !std::str::from_utf8(&body).unwrap().contains("/ws/reload"),
            "index.html should not contain reload script in release mode"
        );
    }
}

// Feature-gated tests for embedded assets
// These tests only run when the `embed-assets` feature is active
// and require frontend/pkg/ and frontend/index.html to exist
#[cfg(all(test, feature = "embed-assets"))]
mod embed_tests {
    use super::*;

    #[test]
    fn embedded_pkg_is_non_empty() {
        assert!(
            EMBEDDED_PKG.entries().count() > 0,
            "embedded pkg/ should contain at least one file"
        );
    }

    #[test]
    fn embedded_pkg_contains_wasm_file() {
        let has_wasm = EMBEDDED_PKG.entries().any(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "wasm")
                .unwrap_or(false)
        });
        assert!(has_wasm, "no .wasm file found in embedded pkg/");
    }

    #[test]
    fn embedded_index_is_non_empty() {
        assert!(
            !EMBEDDED_INDEX.is_empty(),
            "embedded index.html should not be empty"
        );
    }

    #[test]
    fn embedded_index_does_not_contain_reload_script() {
        let html =
            std::str::from_utf8(EMBEDDED_INDEX).expect("embedded index.html should be valid UTF-8");
        assert!(
            !html.contains("/ws/reload"),
            "embedded index.html should not contain reload script reference"
        );
    }
}
