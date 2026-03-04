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
