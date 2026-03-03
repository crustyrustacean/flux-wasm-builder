// tests/serve.rs
//
// Integration tests for static asset serving.
// Run with: cargo test --test serve

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

#[test]
fn scaffolded_backend_main_wires_static_routes() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let main_rs_path = dir.path().join("test-app/backend/src/main.rs");
    let contents = std::fs::read_to_string(&main_rs_path).unwrap();

    // Check route registration order
    // API routes are registered via .configure(api::configure)
    let api_pos = contents
        .find("api::configure")
        .expect("API routes should be registered via api::configure");
    let pkg_pos = contents
        .find("/pkg/{filename}")
        .expect("pkg route should be registered");
    let default_pos = contents
        .find("default_service")
        .expect("default_service should be registered");

    assert!(
        api_pos < pkg_pos,
        "API routes must be registered before pkg routes"
    );
    assert!(
        pkg_pos < default_pos,
        "pkg routes must be registered before default_service"
    );
}

#[test]
fn scaffolded_backend_main_imports_static_assets() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let main_rs_path = dir.path().join("test-app/backend/src/main.rs");
    let contents = std::fs::read_to_string(&main_rs_path).unwrap();

    assert!(
        contents.contains("use build_subsystem::static_assets"),
        "should import static_assets module"
    );
    assert!(
        contents.contains("serve_pkg_file"),
        "should use serve_pkg_file"
    );
    assert!(
        contents.contains("spa_fallback"),
        "should use spa_fallback"
    );
}

#[test]
fn scaffolded_backend_main_shares_config_with_handlers() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let main_rs_path = dir.path().join("test-app/backend/src/main.rs");
    let contents = std::fs::read_to_string(&main_rs_path).unwrap();

    // Check that config is shared via app_data
    assert!(
        contents.contains("app_data(web::Data::new(config.clone()))"),
        "config should be shared via web::Data"
    );
}
