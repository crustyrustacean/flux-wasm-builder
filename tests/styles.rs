// tests/styles.rs

use actix_web::{App, test, web};
use wasm_drydock::dev::styles::styles_handler;
use std::sync::{Arc, RwLock};

#[actix_web::test]
async fn styles_handler_returns_200_with_css_content_type() {
    let css = Arc::new(RwLock::new(b"body { color: red; }".to_vec()));
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(css))
            .route("/styles/screen.css", web::get().to(styles_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/styles/screen.css")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.contains("text/css"),
        "content-type should be text/css, got {ct}"
    );
}

#[actix_web::test]
async fn styles_handler_returns_correct_bytes() {
    let expected = b"h1 { font-size: 2rem; }".to_vec();
    let css = Arc::new(RwLock::new(expected.clone()));
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(css))
            .route("/styles/screen.css", web::get().to(styles_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/styles/screen.css")
        .to_request();
    let body = test::read_body(test::call_service(&app, req).await).await;

    assert_eq!(body.as_ref(), expected.as_slice());
}

#[actix_web::test]
async fn styles_handler_reflects_updated_css() {
    let css = Arc::new(RwLock::new(b"body {}".to_vec()));
    let css_writer = css.clone();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(css))
            .route("/styles/screen.css", web::get().to(styles_handler)),
    )
    .await;

    // Update the CSS in the shared state
    *css_writer.write().unwrap() = b"body { background: blue; }".to_vec();

    let req = test::TestRequest::get()
        .uri("/styles/screen.css")
        .to_request();
    let body = test::read_body(test::call_service(&app, req).await).await;

    assert!(
        std::str::from_utf8(&body).unwrap().contains("blue"),
        "handler should reflect updated CSS bytes"
    );
}
