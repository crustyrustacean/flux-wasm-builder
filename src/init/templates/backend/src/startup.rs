// src/startup.rs

use crate::api;
use crate::configuration::Settings;
use actix_web::{App, HttpServer};
use actix_web::dev::Server;
#[cfg(feature = "embed-assets")]
use actix_web::web;
use actix_web::web::Data;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let port = configuration.application.effective_port();
        let address = format!("{}:{}", configuration.application.host, port);
        let listener = TcpListener::bind(address)?;
        let actual_port = listener.local_addr()?.port();
        let server = run(listener, configuration.application.base_url).await?;
        Ok(Self { port: actual_port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub struct ApplicationBaseUrl(pub String);

async fn run(listener: TcpListener, base_url: String) -> Result<Server, anyhow::Error> {
    let base_url = Data::new(ApplicationBaseUrl(base_url));

    let server = HttpServer::new(move || {
        let app = App::new()
            .wrap(TracingLogger::default())
            .app_data(base_url.clone())
            .configure(api::configure);

        // In release mode, wire in static asset serving
        #[cfg(feature = "embed-assets")]
        let app = app
            .service(
                web::resource("/pkg/{filename}")
                    .route(web::get().to(crate::static_assets::serve_pkg_file))
            )
            .route(
                "/styles/screen.css",
                web::get().to(crate::static_assets::serve_css)
            )
            .default_service(web::to(crate::static_assets::spa_fallback));

        app
    })
    .listen(listener)?
    .run();

    Ok(server)
}
