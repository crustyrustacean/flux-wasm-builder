use actix_web::{web, App, HttpServer};
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
