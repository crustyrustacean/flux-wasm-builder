#[cfg(feature = "embed-assets")]
use actix_web::{HttpResponse, Responder, web};

#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

#[cfg(feature = "embed-assets")]
static EMBEDDED_INDEX: &[u8] = include_bytes!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/index.html")
);

#[cfg(feature = "embed-assets")]
static EMBEDDED_CSS: &[u8] = include_bytes!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/styles/screen.css")
);

#[cfg(feature = "embed-assets")]
static EMBEDDED_PUBLIC: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/public");

#[cfg(feature = "embed-assets")]
pub async fn serve_pkg_file(path: web::Path<String>) -> impl Responder {
    let filename = path.into_inner();
    let safe_name = match std::path::Path::new(&filename).file_name() {
        Some(n) => n.to_owned(),
        None => return HttpResponse::BadRequest().body("invalid filename"),
    };
    match EMBEDDED_PKG.get_file(&safe_name) {
        Some(file) => {
            let mime = mime_guess::from_path(&safe_name).first_or_octet_stream();
            HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(file.contents().to_vec())
        }
        None => HttpResponse::NotFound().finish(),
    }
}

#[cfg(feature = "embed-assets")]
pub async fn serve_css() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .body(EMBEDDED_CSS.to_vec())
}

#[cfg(feature = "embed-assets")]
pub async fn spa_fallback(req: actix_web::HttpRequest) -> impl Responder {
    let req_path = req.path().trim_start_matches('/');
    if !req_path.is_empty() {
        let safe_name = match std::path::Path::new(req_path).file_name() {
            Some(n) => n.to_owned(),
            None => return HttpResponse::BadRequest().finish(),
        };
        if let Some(file) = EMBEDDED_PUBLIC.get_file(&safe_name) {
            let mime = mime_guess::from_path(&safe_name).first_or_octet_stream();
            return HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(file.contents().to_vec());
        }
    }

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(EMBEDDED_INDEX.to_vec())
}
