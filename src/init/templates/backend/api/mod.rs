use actix_web::{web, HttpResponse, Responder};
use {{crate_name}}_shared::{HelloResponse, StatusResponse};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health_check", web::get().to(health_check))
            .route("/hello", web::get().to(hello))
            .route("/status", web::get().to(status))
    );
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().json(HelloResponse {
        message: "Hello from Flux!".to_string(),
    })
}

async fn status() -> impl Responder {
    HttpResponse::Ok().json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0,
    })
}