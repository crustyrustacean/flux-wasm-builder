use actix_web::{web, App, HttpServer};
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

#[cfg(not(feature = "embed-assets"))]
async fn run_server(
    config: BuildConfig,
    port: u16,
    reload_tx: Option<broadcast::Sender<()>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(DevMode(true)))
            .configure(api::configure)
            .route("/pkg/{filename}", web::get().to(serve_pkg_file));

        // Dev-mode only: WebSocket reload endpoint
        if let Some(tx) = reload_tx.as_ref() {
            app = app
                .app_data(web::Data::new(tx.clone()))
                .route("/ws/reload", web::get().to(ws_reload_handler));
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

#[cfg(feature = "embed-assets")]
async fn run_server(
    config: BuildConfig,
    port: u16,
    _reload_tx: Option<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(DevMode(false)))
            .configure(api::configure)
            .route("/pkg/{filename}", web::get().to(serve_pkg_file))
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