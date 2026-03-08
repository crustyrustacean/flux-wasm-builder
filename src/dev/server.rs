// src/dev/server.rs

// dependencies
use crate::DevMode;
use crate::build::{BuildConfig, serve_pkg_file, spa_fallback, ws_reload_handler};
use crate::dev::DevError;
use crate::dev::proxy::proxy_handler;
use crate::dev::styles::styles_handler;
use crate::domain::DrydockConfig;
use actix_web::web::{self, Data};
use actix_web::{App, HttpServer};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

pub async fn serve(
    config: &DrydockConfig,
    build_config: BuildConfig,
    reload_tx: broadcast::Sender<()>,
    css_bytes: Arc<RwLock<Vec<u8>>>,
) -> Result<(), DevError> {
    let client = Data::new(reqwest::Client::new());
    let config_data = Data::new(config.clone());
    let build_config_data = Data::new(build_config);
    let reload_tx_data = Data::new(reload_tx);
    let dev_mode_data = Data::new(DevMode(true));
    let css_data = Data::new(css_bytes);

    HttpServer::new(move || {
        App::new()
            .app_data(client.clone())
            .app_data(config_data.clone())
            .app_data(build_config_data.clone())
            .app_data(reload_tx_data.clone())
            .app_data(dev_mode_data.clone())
            .app_data(css_data.clone())
            .service(web::scope("/api").default_service(web::to(proxy_handler)))
            .service(web::resource("/pkg/{filename}").route(web::get().to(serve_pkg_file)))
            .route("/ws/reload", web::get().to(ws_reload_handler))
            .route("/styles/screen.css", web::get().to(styles_handler))
            .default_service(web::to(spa_fallback))
    })
    .bind(("127.0.0.1", config.dev.public_port))?
    .run()
    .await?;

    Ok(())
}
