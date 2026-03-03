use actix_web::{web, HttpResponse, Responder};

use super::build::BuildConfig;
use super::DevMode;

// Dev-mode only import for reload script injection
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
///
/// # Security
/// Uses `file_name()` to extract only the final path component,
/// preventing path traversal attacks like `/pkg/../../etc/passwd`.
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