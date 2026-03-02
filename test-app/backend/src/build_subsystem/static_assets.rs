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
