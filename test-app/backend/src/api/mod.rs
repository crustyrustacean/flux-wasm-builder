use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use test_app_shared::HelloResponse;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/hello", web::get().to(hello))
            .route("/status", web::get().to(status))
    );
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().json(HelloResponse {
        message: "Hello from the backend!".to_string(),
    })
}

async fn status() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "status": "ok"
    }))
}
