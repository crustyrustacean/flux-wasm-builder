use actix_web::{App, HttpServer, web};
use tracing_subscriber::{fmt, EnvFilter};

mod api;

#[cfg(feature = "embed-assets")]
mod static_assets;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let port: u16 = std::env::var("FLUX_BACKEND_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3001);

    HttpServer::new(move || {
        let app = App::new()
            .configure(api::configure);

        #[cfg(feature = "embed-assets")]
        let app = app
            .service(web::resource("/pkg/{filename}").route(web::get().to(static_assets::serve_pkg_file)))
            .route("/styles/screen.css", web::get().to(static_assets::serve_css))
            .default_service(web::to(static_assets::spa_fallback));

        app
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await?;

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
