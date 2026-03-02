// tests/build.rs
//
// Integration tests for the build subsystem.
// Run with: RUN_WASM_TESTS=1 cargo test --test build

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

fn wasm_tests_enabled() -> bool {
    std::env::var("RUN_WASM_TESTS").is_ok()
}

#[test]
fn wasm_pack_builds_scaffolded_frontend() {
    if !wasm_tests_enabled() {
        println!("Skipping wasm-pack test. Set RUN_WASM_TESTS=1 to enable.");
        return;
    }

    let dir = tempdir().unwrap();

    // Scaffold the project
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Run wasm-pack on the frontend
    let status = std::process::Command::new("wasm-pack")
        .args(["build", "--target", "web", "--dev"])
        .current_dir(dir.path().join("test-app/frontend"))
        .status()
        .expect("failed to invoke wasm-pack");

    assert!(status.success());
    assert!(dir.path().join("test-app/frontend/pkg").exists());
}

#[test]
fn scaffolded_backend_has_build_subsystem() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Check that build.rs exists and has the expected content
    let build_rs_path = dir.path().join("test-app/backend/src/build_subsystem/build.rs");
    assert!(build_rs_path.exists());

    let contents = std::fs::read_to_string(&build_rs_path).unwrap();
    assert!(contents.contains("BuildConfig"));
    assert!(contents.contains("BuildError"));
    assert!(contents.contains("run_wasm_pack"));
}

#[test]
fn scaffolded_backend_main_calls_build_subsystem() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let main_rs_path = dir.path().join("test-app/backend/src/main.rs");
    let contents = std::fs::read_to_string(&main_rs_path).unwrap();

    assert!(contents.contains("use build_subsystem::build"));
    assert!(contents.contains("run_wasm_pack"));
    assert!(contents.contains("initial frontend build"));
}

#[test]
fn scaffolded_backend_exits_on_build_failure() {
    let dir = tempdir().unwrap();

    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let main_rs_path = dir.path().join("test-app/backend/src/main.rs");
    let contents = std::fs::read_to_string(&main_rs_path).unwrap();

    // Check that build failure results in exit(1)
    assert!(contents.contains("std::process::exit(1)"));
    assert!(contents.contains("initial build failed"));
}