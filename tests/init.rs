use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn init_creates_project_in_current_directory() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
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
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("test-project"));
}

#[test]
fn init_prints_wasm_drydock_dev_next_step() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("wasm-drydock dev"));
}

#[test]
fn init_fails_clearly_if_directory_exists() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("test-project")).unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(contains("already exists"));
}

#[test]
fn init_requires_name_argument() {
    cargo_bin_cmd!("wasm-drydock")
        .args(["init"])
        .assert()
        .failure();
}

#[test]
fn init_creates_styles_directory_and_screen_scss() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("test-app/frontend/styles").exists());
    assert!(
        dir.path()
            .join("test-app/frontend/styles/screen.scss")
            .exists()
    );
}

#[test]
fn init_creates_static_assets_rs() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(
        dir.path()
            .join("test-app/backend/src/static_assets.rs")
            .exists()
    );
}

#[test]
fn init_screen_scss_is_non_empty() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let contents =
        std::fs::read_to_string(dir.path().join("test-app/frontend/styles/screen.scss")).unwrap();
    assert!(!contents.is_empty(), "screen.scss should not be empty");
    assert!(
        contents.contains("box-sizing"),
        "screen.scss should contain CSS reset"
    );
}

#[test]
fn init_static_assets_rs_has_embed_assets_cfg_gate() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let contents =
        std::fs::read_to_string(dir.path().join("test-app/backend/src/static_assets.rs")).unwrap();
    assert!(
        contents.contains("embed-assets"),
        "static_assets.rs should be cfg-gated on embed-assets"
    );
    assert!(
        contents.contains("spa_fallback"),
        "static_assets.rs should contain spa_fallback handler"
    );
    assert!(
        contents.contains("serve_pkg_file"),
        "static_assets.rs should contain serve_pkg_file handler"
    );
    assert!(
        contents.contains("serve_css"),
        "static_assets.rs should contain serve_css handler"
    );
}

#[test]
fn init_index_html_links_screen_css() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let contents =
        std::fs::read_to_string(dir.path().join("test-app/frontend/index.html")).unwrap();
    assert!(
        contents.contains("/styles/screen.css"),
        "index.html should link to /styles/screen.css"
    );
}

#[test]
fn init_gitignore_excludes_compiled_css() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let contents = std::fs::read_to_string(dir.path().join("test-app/.gitignore")).unwrap();
    assert!(
        contents.contains("screen.css"),
        ".gitignore should exclude compiled screen.css"
    );
}

#[test]
fn init_creates_public_directory() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("test-app/frontend/public").exists());
}

#[test]
fn init_public_directory_contains_gitkeep() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(
        dir.path()
            .join("test-app/frontend/public/.gitkeep")
            .exists()
    );
}

#[test]
fn dev_command_exists() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["dev"])
        .current_dir(dir.path())
        .assert()
        .failure(); // Fails because no drydock.toml exists
}

#[test]
fn init_creates_dockerfile_by_default() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("test-app/Dockerfile").exists());
}

#[test]
fn init_creates_dockerignore_by_default() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("test-app/.dockerignore").exists());
}

#[test]
fn init_creates_fly_toml_by_default() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("test-app/fly.toml").exists());
}

#[test]
fn init_no_deploy_skips_deployment_files() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app", "--no-deploy"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(!dir.path().join("test-app/Dockerfile").exists());
    assert!(!dir.path().join("test-app/.dockerignore").exists());
    assert!(!dir.path().join("test-app/fly.toml").exists());
}

#[test]
fn init_dockerfile_uses_multi_stage_build() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let dockerfile =
        std::fs::read_to_string(dir.path().join("test-app/Dockerfile")).unwrap();
    assert!(dockerfile.contains("FROM chef AS planner"));
    assert!(dockerfile.contains("FROM chef AS builder"));
    assert!(dockerfile.contains("FROM debian:bookworm-slim AS runtime"));
}

#[test]
fn init_fly_toml_contains_app_name() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "my-awesome-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let fly_toml =
        std::fs::read_to_string(dir.path().join("my-awesome-app/fly.toml")).unwrap();
    assert!(fly_toml.contains("my-awesome-app"));
}

#[test]
fn init_prints_deploy_instructions() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("fly launch"))
        .stdout(contains("fly deploy"));
}

#[test]
fn init_no_deploy_does_not_print_deploy_instructions() {
    let dir = tempdir().unwrap();
    let output = cargo_bin_cmd!("wasm-drydock")
        .args(["init", "test-app", "--no-deploy"])
        .current_dir(dir.path())
        .assert()
        .success();

    // The output should not contain fly deploy instructions
    let output_str = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(!output_str.contains("fly launch"));
    assert!(!output_str.contains("fly deploy"));
}
