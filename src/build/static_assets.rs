// src/build/static_assets.rs

//! Static asset serving handlers for the dev server.
//!
//! This module provides handlers for serving:
//! - Files from the `pkg/` directory (WASM, JS, and other build artifacts)
//! - SPA fallback to `index.html` for client-side routing

use actix_web::{web, HttpResponse, Responder};

use super::build::BuildConfig;

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
pub async fn spa_fallback(config: web::Data<BuildConfig>) -> impl Responder {
    let span = tracing::debug_span!("spa_fallback");
    let _enter = span.enter();

    match tokio::fs::read(&config.index_html_path).await {
        Ok(bytes) => {
            tracing::debug!(path = %config.index_html_path.display(), "serving index.html");
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
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
                .route("/pkg/{filename}", web::get().to(serve_pkg_file)),
        )
        .await;
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
                .route("/pkg/{filename}", web::get().to(serve_pkg_file)),
        )
        .await;
        // URL-encoded traversal: /pkg/..%2Fsecret.txt
        let req = test::TestRequest::get()
            .uri("/pkg/..%2Fsecret.txt")
            .to_request();
        let status = test::call_service(&app, req).await.status();
        assert_ne!(
            status.as_u16(),
            200,
            "path traversal must not return 200"
        );
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
                .default_service(web::get().to(spa_fallback)),
        )
        .await;
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
                .default_service(web::get().to(spa_fallback)),
        )
        .await;
        let req = test::TestRequest::get().uri("/api/hello").to_request();
        let resp = test::call_service(&app, req).await;
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(
            !ct.contains("text/html"),
            "API route returned index.html instead of JSON"
        );
    }
}
