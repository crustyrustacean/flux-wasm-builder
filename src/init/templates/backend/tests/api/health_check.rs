// tests/api/health_check.rs

use crate::helpers::spawn_app;
use {{ crate_name }}_backend::response::ApiResponse;

#[tokio::test]
async fn health_check_returns_200_with_success_body() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());

    let body: ApiResponse<()> = response
        .json()
        .await
        .expect("Failed to deserialize response.");

    assert!(body.success);
    assert!(body.error.is_none());
}
