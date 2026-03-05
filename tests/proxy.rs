// tests/proxy.rs

use actix_web::{App, HttpResponse, test, web};
use flux_wasm_builder::dev::proxy::proxy_handler;
use flux_wasm_builder::domain::{DevConfig, FluxConfig, ProjectConfig};

fn make_config(backend_port: u16) -> FluxConfig {
    FluxConfig {
        project: ProjectConfig {
            name: "test".to_string(),
        },
        dev: DevConfig {
            public_port: 8080,
            backend_port,
            watch_debounce_ms: 300,
        },
    }
}

#[actix_web::test]
async fn proxy_returns_502_when_backend_unreachable() {
    // Port 19999 should have nothing listening
    let config = make_config(19999);
    let client = reqwest::Client::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(client))
            .app_data(web::Data::new(config))
            .service(web::scope("/api").default_service(web::to(proxy_handler))),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/hello").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 502);
}

#[actix_web::test]
async fn proxy_forwards_request_and_returns_backend_response() {
    // Spin up a mock backend on a fixed port
    let mock_port: u16 = 19998;
    let mock_server = actix_web::HttpServer::new(|| {
        actix_web::App::new().route(
            "/api/hello",
            web::get().to(|| async { HttpResponse::Ok().body("hello from backend") }),
        )
    })
    .bind(format!("127.0.0.1:{mock_port}"))
    .unwrap();

    let server = mock_server.run();
    let handle = tokio::spawn(server);

    // Brief yield to let the server start accepting connections
    tokio::task::yield_now().await;

    let config = make_config(mock_port);
    let client = reqwest::Client::new();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(client))
            .app_data(web::Data::new(config))
            .service(web::scope("/api").default_service(web::to(proxy_handler))),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/hello").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    assert_eq!(body, "hello from backend");

    handle.abort();
}
