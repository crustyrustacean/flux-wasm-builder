// tests/reload.rs
//
// Integration tests for reload functionality.
// Run with: cargo test --test reload

use actix_web::{App, test as actix_test, web};
use tempfile::tempdir;
use tokio::sync::broadcast;
use wasm_drydock::{
    BuildConfig, DevMode, RELOAD_SCRIPT, inject_reload_script, spa_fallback, ws_reload_handler,
};

// =============================================================================
// Unit Tests for inject_reload_script
// =============================================================================

#[test]
fn script_is_injected_before_body_close_tag() {
    let html = "<html><body><p>Hello</p></body></html>";
    let result = inject_reload_script(html);
    let script_pos = result.find(RELOAD_SCRIPT).expect("script not found");
    let body_pos = result.find("</body>").expect("</body> not found");
    assert!(
        script_pos < body_pos,
        "script should be injected before </body>, but script is at {} and </body> is at {}",
        script_pos,
        body_pos
    );
}

#[test]
fn handles_html_without_body_tag_gracefully() {
    let html = "<html><p>No body tag</p></html>";
    let result = inject_reload_script(html);
    assert!(
        result.contains(RELOAD_SCRIPT),
        "script should be appended when </body> is absent"
    );
}

#[test]
fn reload_script_references_ws_reload_path() {
    assert!(
        RELOAD_SCRIPT.contains("/ws/reload"),
        "reload script must reference /ws/reload WebSocket path"
    );
}

#[test]
fn reload_script_contains_onclose_reconnect() {
    assert!(
        RELOAD_SCRIPT.contains("onclose"),
        "reload script must include reconnect logic — see Section 4.3 of spec"
    );
}

#[test]
fn inject_reload_script_preserves_original_content() {
    let html = "<html><body><p>Hello World</p></body></html>";
    let result = inject_reload_script(html);
    assert!(
        result.contains("<p>Hello World</p>"),
        "original content should be preserved"
    );
}

#[test]
fn broadcast_send_with_no_receivers_does_not_panic() {
    let (tx, _rx) = broadcast::channel::<()>(16);
    drop(_rx);
    let result = tx.send(());
    // The send returns Err when there are no receivers
    // This is expected and acceptable behavior
    assert!(result.is_err(), "send should return Err when no receivers");
}

#[test]
fn broadcast_send_with_receivers_succeeds() {
    let (tx, _rx) = broadcast::channel::<()>(16);
    let result = tx.send(());
    assert!(result.is_ok(), "send should succeed when receivers exist");
}

// =============================================================================
// Integration Tests for WebSocket Endpoint
// =============================================================================

#[actix_web::test]
async fn ws_endpoint_returns_101_switching_protocols() {
    let (tx, _rx) = broadcast::channel::<()>(16);
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(tx))
            .route("/ws/reload", web::get().to(ws_reload_handler)),
    )
    .await;
    let req = actix_test::TestRequest::get()
        .uri("/ws/reload")
        .insert_header(("upgrade", "websocket"))
        .insert_header(("connection", "upgrade"))
        .insert_header(("sec-websocket-version", "13"))
        .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
        .to_request();
    assert_eq!(actix_test::call_service(&app, req).await.status(), 101);
}

// =============================================================================
// Integration Tests for DevMode Script Injection
// =============================================================================

#[actix_web::test]
async fn index_html_contains_reload_script_when_dev_mode_true() {
    let dir = tempdir().unwrap();
    let index = dir.path().join("index.html");
    std::fs::write(&index, b"<html><body></body></html>").unwrap();
    let public = dir.path().join("public");
    std::fs::create_dir(&public).unwrap();
    let config = BuildConfig {
        index_html_path: index,
        public_path: public,
        ..BuildConfig::new("/unused")
    };
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(DevMode(true)))
            .default_service(web::to(spa_fallback)),
    )
    .await;
    let body = actix_test::read_body(
        actix_test::call_service(&app, actix_test::TestRequest::get().uri("/").to_request()).await,
    )
    .await;
    assert!(
        std::str::from_utf8(&body).unwrap().contains("/ws/reload"),
        "index.html should contain reload script in dev mode"
    );
}

#[actix_web::test]
async fn index_html_omits_reload_script_when_dev_mode_false() {
    let dir = tempdir().unwrap();
    let index = dir.path().join("index.html");
    std::fs::write(&index, b"<html><body></body></html>").unwrap();
    let public = dir.path().join("public");
    std::fs::create_dir(&public).unwrap();
    let config = BuildConfig {
        index_html_path: index,
        public_path: public,
        ..BuildConfig::new("/unused")
    };
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(DevMode(false)))
            .default_service(web::to(spa_fallback)),
    )
    .await;
    let body = actix_test::read_body(
        actix_test::call_service(&app, actix_test::TestRequest::get().uri("/").to_request()).await,
    )
    .await;
    assert!(
        !std::str::from_utf8(&body).unwrap().contains("/ws/reload"),
        "index.html should not contain reload script in release mode"
    );
}
