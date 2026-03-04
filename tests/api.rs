use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

#[test]
fn scaffolded_api_mod_imports_status_response() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let api_mod =
        std::fs::read_to_string(dir.path().join("test-app/backend/src/api/mod.rs")).unwrap();

    assert!(
        api_mod.contains("StatusResponse"),
        "api/mod.rs should import StatusResponse from shared crate"
    );
    assert!(
        api_mod.contains("HelloResponse"),
        "api/mod.rs should import HelloResponse from shared crate"
    );
    // Should NOT use serde_json::json! for status endpoint
    assert!(
        !api_mod.contains("json!"),
        "status handler should use StatusResponse, not json!()"
    );
}

#[test]
fn scaffolded_api_status_uses_shared_type() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let api_mod =
        std::fs::read_to_string(dir.path().join("test-app/backend/src/api/mod.rs")).unwrap();

    assert!(
        api_mod.contains("StatusResponse {"),
        "status handler should construct a StatusResponse struct"
    );
    assert!(
        api_mod.contains("CARGO_PKG_VERSION"),
        "status handler should use CARGO_PKG_VERSION for version"
    );
    assert!(
        api_mod.contains("uptime_seconds"),
        "status handler should include uptime_seconds field"
    );
}

#[test]
fn scaffolded_shared_lib_has_compile_time_assertions() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let shared_lib =
        std::fs::read_to_string(dir.path().join("test-app/shared/src/lib.rs")).unwrap();

    assert!(
        shared_lib.contains("assert_serialize"),
        "shared lib should have compile-time serialize assertion"
    );
    assert!(
        shared_lib.contains("assert_deserialize"),
        "shared lib should have compile-time deserialize assertion"
    );
    assert!(
        shared_lib.contains("const _: fn()"),
        "shared lib should use const fn pattern for compile-time checks"
    );
}

#[test]
fn scaffolded_shared_lib_has_round_trip_tests() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let shared_lib =
        std::fs::read_to_string(dir.path().join("test-app/shared/src/lib.rs")).unwrap();

    assert!(
        shared_lib.contains("hello_response_round_trips_through_json"),
        "shared lib should test HelloResponse round-trip"
    );
    assert!(
        shared_lib.contains("status_response_round_trips_through_json"),
        "shared lib should test StatusResponse round-trip"
    );
    assert!(
        shared_lib.contains("unknown_fields_are_ignored_not_rejected"),
        "shared lib should test unknown field tolerance"
    );
}

#[test]
fn scaffolded_shared_cargo_toml_has_serde_json_dev_dependency() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let cargo_toml =
        std::fs::read_to_string(dir.path().join("test-app/shared/Cargo.toml")).unwrap();

    assert!(
        cargo_toml.contains("[dev-dependencies]"),
        "shared Cargo.toml should have dev-dependencies section"
    );
    assert!(
        cargo_toml.contains("serde_json"),
        "shared Cargo.toml should have serde_json as dev-dependency"
    );
}

#[test]
fn scaffolded_frontend_fetches_api_status() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let frontend_lib =
        std::fs::read_to_string(dir.path().join("test-app/frontend/src/lib.rs")).unwrap();

    assert!(
        frontend_lib.contains("StatusResponse"),
        "frontend should import StatusResponse"
    );
    assert!(
        frontend_lib.contains("/api/status"),
        "frontend should fetch /api/status"
    );
}

#[test]
fn scaffolded_api_responses_use_json_content_type() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let api_mod =
        std::fs::read_to_string(dir.path().join("test-app/backend/src/api/mod.rs")).unwrap();

    // Both handlers use HttpResponse::Ok().json(...) which sets application/json
    assert!(
        api_mod.contains(".json(HelloResponse"),
        "hello handler should use .json() for content-type"
    );
    assert!(
        api_mod.contains(".json(StatusResponse"),
        "status handler should use .json() for content-type"
    );
}
