// tests/release.rs

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn release_command_fails_without_flux_toml() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["release"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn release_command_exists_in_help() {
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["--help"])
        .assert()
        .success()
        .stdout(contains("release"));
}
