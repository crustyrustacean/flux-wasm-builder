// src/dev/styles.rs

use actix_web::{HttpResponse, web};
use std::sync::{Arc, RwLock};

pub async fn styles_handler(css_bytes: web::Data<Arc<RwLock<Vec<u8>>>>) -> HttpResponse {
    let bytes = css_bytes.read().unwrap().clone();
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .body(bytes)
}
