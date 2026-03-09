// src/init/mod.rs

// module declarations
mod templates;

// dependencies
use crate::env_check::{
    EnvCheckError, wasm_pack_on_path, wasm_pack_version_ok, wasm32_target_installed,
};
use crate::validation::validate_project_name;
use std::path::Path;

/// Error type for project scaffolding failures.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("directory '{0}' already exists")]
    AlreadyExists(std::path::PathBuf),

    #[error("environment check failed: {0}")]
    EnvCheck(#[from] EnvCheckError),

    #[error("invalid project name: {0}")]
    InvalidName(#[from] crate::validation::ValidationError),

    #[error("failed to create directory '{path}': {source}")]
    CreateDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write '{path}': {source}")]
    WriteFile {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    #[error("template error: {0}")]
    Template(#[from] crate::init::templates::TemplateError),
}

/// Scaffold a new fullstack project.
///
/// Creates a three-crate Cargo workspace with backend, frontend, and shared crates.
/// The project is created in `root/name/`.
///
/// # Arguments
///
/// * `root` - Parent directory where the project will be created
/// * `name` - Project name (used as directory name and in generated files)
/// * `include_deploy` - If true, generates Dockerfile, .dockerignore, and fly.toml
///
/// # Errors
///
/// Returns `InitError` if:
/// - The project name is invalid (empty, too long, invalid characters, reserved word)
/// - Environment checks fail (wasm-pack not found, wrong version, missing target)
/// - The target directory already exists
/// - Directory or file creation fails
pub fn scaffold(root: &Path, name: &str, include_deploy: bool) -> Result<(), InitError> {
    let span = tracing::info_span!("scaffold", project = name);
    let _enter = span.enter();

    // Step 0: Validate project name
    tracing::debug!("validating project name");
    validate_project_name(name)?;

    // Step 1: Environment check
    tracing::info!("running environment checks");
    if !wasm_pack_on_path() {
        return Err(InitError::EnvCheck(EnvCheckError::WasmPackNotFound));
    }
    wasm_pack_version_ok()?;
    wasm32_target_installed()?;

    // Step 2: Existence check
    let project_root = root.join(name);
    if project_root.exists() {
        tracing::warn!(path = %project_root.display(), "target directory already exists");
        return Err(InitError::AlreadyExists(project_root));
    }

    // Step 3: Directory creation
    tracing::debug!("creating directory structure");
    create_dirs(&project_root)?;

    // Step 4: Write files
    tracing::debug!("writing project files");
    write_files(&project_root, name)?;

    // Step 5: Write deployment files
    if include_deploy {
        tracing::debug!("writing deployment files");
        write_deploy_files(&project_root, name)?;
    }

    // Step 6: Success output
    print_success(name, include_deploy);
    tracing::info!("project created successfully");
    Ok(())
}

fn create_dirs(project_root: &Path) -> Result<(), InitError> {
    let dirs = [
        project_root.join(".cargo"),
        project_root.join("backend/src/bin"),
        project_root.join("backend/src/api"),
        project_root.join("backend/configuration"),
        project_root.join("backend/tests/api"),
        project_root.join("frontend/src"),
        project_root.join("frontend/styles"),
        project_root.join("frontend/public"),
        project_root.join("shared/src"),
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir).map_err(|e| InitError::CreateDir {
            path: dir.clone(),
            source: e,
        })?;
    }

    Ok(())
}

fn write_files(project_root: &Path, name: &str) -> Result<(), InitError> {
    use templates::{ProjectContext, render_template};

    let ctx = ProjectContext::new(name);

    let files: Vec<(std::path::PathBuf, String)> = vec![
        // Workspace root
        (
            project_root.join("Cargo.toml"),
            render_template("workspace/cargo.toml", &ctx)?,
        ),
        (
            project_root.join(".cargo/config.toml"),
            render_template("workspace/cargo-config.toml", &ctx)?,
        ),
        (
            project_root.join(".gitignore"),
            render_template("gitignore", &ctx)?,
        ),
        // Backend
        (
            project_root.join("backend/Cargo.toml"),
            render_template("backend/cargo.toml", &ctx)?,
        ),
        (
            project_root.join("drydock.toml"),
            render_template("backend/drydock.toml", &ctx)?,
        ),
        (
            project_root.join("backend/build.rs"),
            render_template("backend/build.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/bin/main.rs"),
            render_template("backend/src/bin/main.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/lib.rs"),
            render_template("backend/src/lib.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/configuration.rs"),
            render_template("backend/src/configuration.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/error.rs"),
            render_template("backend/src/error.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/response.rs"),
            render_template("backend/src/response.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/telemetry.rs"),
            render_template("backend/src/telemetry.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/startup.rs"),
            render_template("backend/src/startup.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/static_assets.rs"),
            render_template("backend/src/static_assets.rs", &ctx)?,
        ),
        (
            project_root.join("backend/src/api/mod.rs"),
            render_template("backend/api/mod.rs", &ctx)?,
        ),
        (
            project_root.join("backend/configuration/base.yaml"),
            render_template("backend/configuration/base.yaml", &ctx)?,
        ),
        (
            project_root.join("backend/configuration/local.yaml"),
            render_template("backend/configuration/local.yaml", &ctx)?,
        ),
        (
            project_root.join("backend/configuration/production.yaml"),
            render_template("backend/configuration/production.yaml", &ctx)?,
        ),
        (
            project_root.join("backend/tests/api/main.rs"),
            render_template("backend/tests/api/main.rs", &ctx)?,
        ),
        (
            project_root.join("backend/tests/api/helpers.rs"),
            render_template("backend/tests/api/helpers.rs", &ctx)?,
        ),
        (
            project_root.join("backend/tests/api/health_check.rs"),
            render_template("backend/tests/api/health_check.rs", &ctx)?,
        ),
        // Frontend
        (
            project_root.join("frontend/Cargo.toml"),
            render_template("frontend/cargo.toml", &ctx)?,
        ),
        (
            project_root.join("frontend/index.html"),
            render_template("frontend/index.html", &ctx)?,
        ),
        (
            project_root.join("frontend/src/lib.rs"),
            render_template("frontend/lib.rs", &ctx)?,
        ),
        (
            project_root.join("frontend/styles/screen.scss"),
            render_template("frontend/styles/screen.scss", &ctx)?,
        ),
        (project_root.join("frontend/public/.gitkeep"), String::new()),
        // Shared
        (
            project_root.join("shared/Cargo.toml"),
            render_template("shared/cargo.toml", &ctx)?,
        ),
        (
            project_root.join("shared/src/lib.rs"),
            render_template("shared/lib.rs", &ctx)?,
        ),
    ];

    for (path, content) in &files {
        std::fs::write(path, content).map_err(|e| InitError::WriteFile {
            path: path.clone(),
            source: e,
        })?;
    }

    Ok(())
}

fn write_deploy_files(project_root: &Path, name: &str) -> Result<(), InitError> {
    use templates::{ProjectContext, render_template};

    let ctx = ProjectContext::new(name);

    let files: Vec<(std::path::PathBuf, String)> = vec![
        (
            project_root.join("Dockerfile"),
            render_template("deployment/Dockerfile", &ctx)?,
        ),
        (
            project_root.join(".dockerignore"),
            render_template("deployment/dockerignore", &ctx)?,
        ),
        (
            project_root.join("fly.toml"),
            render_template("deployment/fly.toml", &ctx)?,
        ),
    ];

    for (path, content) in &files {
        std::fs::write(path, content).map_err(|e| InitError::WriteFile {
            path: path.clone(),
            source: e,
        })?;
    }

    Ok(())
}

fn print_success(name: &str, include_deploy: bool) {
    println!("✓ Created project: {name}/");
    println!();
    println!("  {name}/");
    println!("  ├── .cargo/config.toml");
    println!("  ├── .gitignore");
    if include_deploy {
        println!("  ├── .dockerignore");
        println!("  ├── Dockerfile");
    }
    println!("  ├── Cargo.toml");
    if include_deploy {
        println!("  ├── fly.toml");
    }
    println!("  ├── backend/");
    println!("  ├── frontend/");
    println!("  └── shared/");
    println!();
    println!("Next steps:");
    println!("  cd {name}");
    println!("  wasm-drydock dev");
    if include_deploy {
        println!();
        println!("Deploy to Fly.io:");
        println!("  fly launch --no-deploy");
        println!("  fly deploy");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_expected_directory_structure() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();

        assert!(root.path().join("my-app/Cargo.toml").exists());
        assert!(root.path().join("my-app/.cargo/config.toml").exists());
        assert!(root.path().join("my-app/.gitignore").exists());
        assert!(root.path().join("my-app/backend/Cargo.toml").exists());
        assert!(root.path().join("my-app/drydock.toml").exists());
        assert!(root.path().join("my-app/backend/build.rs").exists());
        assert!(root.path().join("my-app/backend/src/bin/main.rs").exists());
        assert!(root.path().join("my-app/backend/src/lib.rs").exists());
        assert!(
            root.path()
                .join("my-app/backend/src/static_assets.rs")
                .exists()
        );
        assert!(root.path().join("my-app/frontend/Cargo.toml").exists());
        assert!(root.path().join("my-app/frontend/src/lib.rs").exists());
        assert!(root.path().join("my-app/frontend/index.html").exists());
        assert!(root.path().join("my-app/shared/Cargo.toml").exists());
        assert!(root.path().join("my-app/shared/src/lib.rs").exists());
    }

    #[test]
    fn workspace_cargo_toml_lists_all_members_with_resolver_3() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents = std::fs::read_to_string(root.path().join("my-app/Cargo.toml")).unwrap();
        assert!(contents.contains("backend"));
        assert!(contents.contains("frontend"));
        assert!(contents.contains("shared"));
        assert!(contents.contains("resolver = \"3\""));
        assert!(contents.contains("edition = \"2024\""));
    }

    #[test]
    fn cargo_config_contains_run_alias_for_backend() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/.cargo/config.toml")).unwrap();
        assert!(contents.contains("[alias]"));
        assert!(contents.contains("-p my-app-backend"));
    }

    #[test]
    fn gitignore_excludes_target_and_pkg() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents = std::fs::read_to_string(root.path().join("my-app/.gitignore")).unwrap();
        assert!(contents.contains("target/"));
        assert!(contents.contains("frontend/pkg/"));
        assert!(contents.contains("frontend/styles/screen.css"));
    }

    #[test]
    fn frontend_cargo_toml_sets_cdylib_and_pins_yew_022() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/frontend/Cargo.toml")).unwrap();
        assert!(contents.contains("cdylib"));
        assert!(contents.contains("yew"));
        assert!(contents.contains("0.22.1"));
    }

    #[test]
    fn backend_bin_main_uses_tokio_main() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/src/bin/main.rs")).unwrap();
        assert!(contents.contains("tokio::main"));
    }

    #[test]
    fn backend_has_lib_rs() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        assert!(root.path().join("my-app/backend/src/lib.rs").exists());
    }

    #[test]
    fn backend_has_configuration_directory() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        assert!(
            root.path()
                .join("my-app/backend/configuration/base.yaml")
                .exists()
        );
        assert!(
            root.path()
                .join("my-app/backend/configuration/local.yaml")
                .exists()
        );
        assert!(
            root.path()
                .join("my-app/backend/configuration/production.yaml")
                .exists()
        );
    }

    #[test]
    fn backend_configuration_default_port_is_3001() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/configuration/base.yaml"))
                .unwrap();
        assert!(contents.contains("3001"));
    }

    #[test]
    fn backend_has_startup_module() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        assert!(root.path().join("my-app/backend/src/startup.rs").exists());
    }

    #[test]
    fn backend_has_error_and_response_modules() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        assert!(root.path().join("my-app/backend/src/error.rs").exists());
        assert!(root.path().join("my-app/backend/src/response.rs").exists());
    }

    #[test]
    fn backend_api_health_check_returns_api_response() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/src/api/mod.rs")).unwrap();
        assert!(contents.contains("ApiResponse"));
    }

    #[test]
    fn backend_build_rs_uses_env_var_not_cfg_macro() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/build.rs")).unwrap();
        assert!(contents.contains("CARGO_FEATURE_EMBED_ASSETS"));
        // cfg!() does not work for feature detection in build.rs
        assert!(!contents.contains("cfg!(feature"));
    }

    #[test]
    fn backend_cargo_toml_has_embed_assets_feature_with_include_dir() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/Cargo.toml")).unwrap();
        assert!(contents.contains("embed-assets"));
        assert!(contents.contains("include_dir"));
    }

    #[test]
    fn shared_lib_does_not_use_deny_unknown_fields() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/shared/src/lib.rs")).unwrap();
        assert!(contents.contains("HelloResponse"));
        assert!(contents.contains("Serialize"));
        assert!(contents.contains("Deserialize"));
        assert!(
            !contents.contains("deny_unknown_fields"),
            "shared types must not use deny_unknown_fields — see Section 5.2"
        );
    }

    #[test]
    fn fails_if_directory_already_exists() {
        let root = tempdir().unwrap();
        std::fs::create_dir(root.path().join("my-app")).unwrap();
        let result = scaffold(root.path(), "my-app", true);
        assert!(matches!(result, Err(InitError::AlreadyExists(_))));
    }

    #[test]
    fn project_name_substituted_into_crate_names() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "cool-project", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("cool-project/backend/Cargo.toml")).unwrap();
        assert!(contents.contains("cool-project"));
    }

    #[test]
    fn shared_lib_has_compile_time_trait_assertions() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/shared/src/lib.rs")).unwrap();
        assert!(contents.contains("assert_serialize"));
        assert!(contents.contains("assert_deserialize"));
        assert!(contents.contains("const _: fn()"));
    }

    #[test]
    fn shared_cargo_toml_has_serde_json_dev_dependency() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/shared/Cargo.toml")).unwrap();
        assert!(contents.contains("[dev-dependencies]"));
        assert!(contents.contains("serde_json"));
    }

    #[test]
    fn backend_api_uses_status_response_not_json_macro() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/src/api/mod.rs")).unwrap();
        assert!(contents.contains("StatusResponse"));
        assert!(
            !contents.contains("json!"),
            "should use StatusResponse struct, not json!()"
        );
    }

    #[test]
    fn frontend_imports_and_fetches_status_response() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/frontend/src/lib.rs")).unwrap();
        assert!(contents.contains("StatusResponse"));
        assert!(contents.contains("/api/status"));
    }

    #[test]
    fn fails_if_project_name_is_empty() {
        let root = tempdir().unwrap();
        let result = scaffold(root.path(), "", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));
    }

    #[test]
    fn fails_if_project_name_has_invalid_characters() {
        let root = tempdir().unwrap();
        let result = scaffold(root.path(), "test app", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));

        let result = scaffold(root.path(), "test\"app", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));

        let result = scaffold(root.path(), "test/app", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));
    }

    #[test]
    fn fails_if_project_name_is_reserved_word() {
        let root = tempdir().unwrap();
        let result = scaffold(root.path(), "test", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));

        let result = scaffold(root.path(), "build", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));

        let result = scaffold(root.path(), "fn", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));
    }

    #[test]
    fn backend_configuration_rs_has_find_configuration_dir() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/src/configuration.rs"))
                .unwrap();
        assert!(
            contents.contains("find_configuration_dir"),
            "configuration.rs should have find_configuration_dir function"
        );
    }

    #[test]
    fn backend_configuration_rs_has_effective_port() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/src/configuration.rs"))
                .unwrap();
        assert!(
            contents.contains("effective_port"),
            "configuration.rs should have effective_port method"
        );
        assert!(
            contents.contains("DRYDOCK_BACKEND_PORT"),
            "configuration.rs should respect DRYDOCK_BACKEND_PORT env var"
        );
    }

    #[test]
    fn fails_if_project_name_starts_with_invalid_char() {
        let root = tempdir().unwrap();
        let result = scaffold(root.path(), "-test", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));

        let result = scaffold(root.path(), "_test", true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));
    }

    #[test]
    fn fails_if_project_name_too_long() {
        let root = tempdir().unwrap();
        let long_name = "a".repeat(65);
        let result = scaffold(root.path(), &long_name, true);
        assert!(matches!(result, Err(InitError::InvalidName(_))));
    }

    #[test]
    fn creates_dockerfile_when_include_deploy_is_true() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        assert!(root.path().join("my-app/Dockerfile").exists());
    }

    #[test]
    fn creates_dockerignore_when_include_deploy_is_true() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        assert!(root.path().join("my-app/.dockerignore").exists());
    }

    #[test]
    fn creates_fly_toml_when_include_deploy_is_true() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        assert!(root.path().join("my-app/fly.toml").exists());
    }

    #[test]
    fn skips_deployment_files_when_include_deploy_is_false() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", false).unwrap();
        assert!(!root.path().join("my-app/Dockerfile").exists());
        assert!(!root.path().join("my-app/.dockerignore").exists());
        assert!(!root.path().join("my-app/fly.toml").exists());
    }

    #[test]
    fn dockerfile_uses_multi_stage_build() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let dockerfile = std::fs::read_to_string(root.path().join("my-app/Dockerfile")).unwrap();
        assert!(dockerfile.contains("FROM chef AS planner"));
        assert!(dockerfile.contains("FROM chef AS builder"));
        assert!(dockerfile.contains("FROM debian:bookworm-slim AS runtime"));
    }

    #[test]
    fn dockerfile_contains_project_name() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "test-project", true).unwrap();
        let dockerfile =
            std::fs::read_to_string(root.path().join("test-project/Dockerfile")).unwrap();
        assert!(dockerfile.contains("test-project-backend"));
    }

    #[test]
    fn fly_toml_contains_app_name() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-cool-app", true).unwrap();
        let fly_toml = std::fs::read_to_string(root.path().join("my-cool-app/fly.toml")).unwrap();
        assert!(fly_toml.contains("my-cool-app"));
    }

    #[test]
    fn fly_toml_has_correct_internal_port() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app", true).unwrap();
        let fly_toml = std::fs::read_to_string(root.path().join("my-app/fly.toml")).unwrap();
        assert!(fly_toml.contains("internal_port = 3001"));
    }
}
