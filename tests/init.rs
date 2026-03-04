use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn init_creates_project_in_current_directory() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("test-project/Cargo.toml").exists());
    assert!(dir.path().join("test-project/.cargo/config.toml").exists());
    assert!(dir.path().join("test-project/.gitignore").exists());
}

#[test]
fn init_prints_success_message_with_project_name() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("test-project"));
}

#[test]
fn init_prints_cargo_backend_next_step() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("cargo backend"));
}

#[test]
fn init_fails_clearly_if_directory_exists() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("test-project")).unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(contains("already exists"));
}

#[test]
fn init_requires_name_argument() {
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init"])
        .assert()
        .failure();
}

#[test]
fn dev_command_exists() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["dev"])
        .current_dir(dir.path())
        .assert()
        .failure(); // Fails because no flux.toml exists
}
