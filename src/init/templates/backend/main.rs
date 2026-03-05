use actix_web::{App, HttpServer};
use tracing_subscriber::{fmt, EnvFilter};

mod api;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let port: u16 = std::env::var("FLUX_BACKEND_PORT")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(3001);

    HttpServer::new(move || {
        App::new()
            .configure(api::configure)
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