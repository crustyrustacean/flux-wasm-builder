// src/init/mod.rs

// module declarations
mod templates;

// dependencies
use std::path::Path;
use crate::env_check::{
    wasm32_target_installed, wasm_pack_on_path, wasm_pack_version_ok, EnvCheckError,
};

/// Error type for project scaffolding failures.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("directory '{0}' already exists")]
    AlreadyExists(std::path::PathBuf),

    #[error("environment check failed: {0}")]
    EnvCheck(#[from] EnvCheckError),

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
}

/// Scaffold a new fullstack project.
///
/// Creates a three-crate Cargo workspace with backend, frontend, and shared crates.
/// The project is created in `root/name/`.
///
/// # Errors
///
/// Returns `InitError` if:
/// - Environment checks fail (wasm-pack not found, wrong version, missing target)
/// - The target directory already exists
/// - Directory or file creation fails
pub fn scaffold(root: &Path, name: &str) -> Result<(), InitError> {
    let span = tracing::info_span!("scaffold", project = name);
    let _enter = span.enter();

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

    // Step 5: Success output
    print_success(name);
    tracing::info!("project created successfully");
    Ok(())
}

fn create_dirs(project_root: &Path) -> Result<(), InitError> {
    let dirs = [
        project_root.join(".cargo"),
        project_root.join("backend/src/build_subsystem"),
        project_root.join("backend/src/api"),
        project_root.join("frontend/src"),
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
    let files: Vec<(std::path::PathBuf, String)> = vec![
        // Workspace root
        (
            project_root.join("Cargo.toml"),
            templates::workspace_cargo_toml(name),
        ),
        (
            project_root.join(".cargo/config.toml"),
            templates::cargo_config(name),
        ),
        (
            project_root.join(".gitignore"),
            templates::gitignore().to_string(),
        ),
        // Backend
        (
            project_root.join("backend/Cargo.toml"),
            templates::backend_cargo_toml(name),
        ),
        (
            project_root.join("backend/build.rs"),
            templates::backend_build_rs().to_string(),
        ),
        (
            project_root.join("backend/src/main.rs"),
            templates::backend_main_rs().to_string(),
        ),
        (
            project_root.join("backend/src/build_subsystem/mod.rs"),
            templates::backend_build_subsystem_mod().to_string(),
        ),
        (
            project_root.join("backend/src/build_subsystem/build.rs"),
            templates::backend_build_subsystem_build_rs().to_string(),
        ),
        (
            project_root.join("backend/src/build_subsystem/build_coordinator.rs"),
            templates::backend_build_subsystem_build_coordinator_rs().to_string(),
        ),
        (
            project_root.join("backend/src/build_subsystem/watcher.rs"),
            templates::backend_build_subsystem_watcher_rs().to_string(),
        ),
        (
            project_root.join("backend/src/build_subsystem/reload.rs"),
            templates::stub_file().to_string(),
        ),
        (
            project_root.join("backend/src/build_subsystem/static_assets.rs"),
            templates::backend_build_subsystem_static_assets_rs().to_string(),
        ),
        (
            project_root.join("backend/src/api/mod.rs"),
            templates::backend_api_mod(name),
        ),
        // Frontend
        (
            project_root.join("frontend/Cargo.toml"),
            templates::frontend_cargo_toml(name),
        ),
        (
            project_root.join("frontend/index.html"),
            templates::frontend_index_html(name),
        ),
        (
            project_root.join("frontend/src/lib.rs"),
            templates::frontend_lib_rs(name),
        ),
        // Shared
        (
            project_root.join("shared/Cargo.toml"),
            templates::shared_cargo_toml(name),
        ),
        (
            project_root.join("shared/src/lib.rs"),
            templates::shared_lib_rs().to_string(),
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

fn print_success(name: &str) {
    println!("✓ Created project: {name}/");
    println!();
    println!("  {name}/");
    println!("  ├── .cargo/config.toml");
    println!("  ├── .gitignore");
    println!("  ├── Cargo.toml");
    println!("  ├── backend/");
    println!("  ├── frontend/");
    println!("  └── shared/");
    println!();
    println!("Next steps:");
    println!("  cd {name}");
    println!("  cargo backend");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_expected_directory_structure() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();

        assert!(root.path().join("my-app/Cargo.toml").exists());
        assert!(root.path().join("my-app/.cargo/config.toml").exists());
        assert!(root.path().join("my-app/.gitignore").exists());
        assert!(root.path().join("my-app/backend/Cargo.toml").exists());
        assert!(root.path().join("my-app/backend/build.rs").exists());
        assert!(root.path().join("my-app/backend/src/main.rs").exists());
        assert!(root.path().join("my-app/frontend/Cargo.toml").exists());
        assert!(root.path().join("my-app/frontend/src/lib.rs").exists());
        assert!(root.path().join("my-app/frontend/index.html").exists());
        assert!(root.path().join("my-app/shared/Cargo.toml").exists());
        assert!(root.path().join("my-app/shared/src/lib.rs").exists());
    }

    #[test]
    fn workspace_cargo_toml_lists_all_members_with_resolver_3() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
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
        scaffold(root.path(), "my-app").unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/.cargo/config.toml")).unwrap();
        assert!(contents.contains("[alias]"));
        assert!(contents.contains("-p my-app-backend"));
    }

    #[test]
    fn gitignore_excludes_target_and_pkg() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(root.path().join("my-app/.gitignore")).unwrap();
        assert!(contents.contains("target/"));
        assert!(contents.contains("frontend/pkg/"));
    }

    #[test]
    fn frontend_cargo_toml_sets_cdylib_and_pins_yew_021() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/frontend/Cargo.toml")).unwrap();
        assert!(contents.contains("cdylib"));
        assert!(contents.contains("yew"));
        assert!(contents.contains("0.22.1"));
    }

    #[test]
    fn backend_main_uses_actix_web_main_not_tokio_main() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/src/main.rs")).unwrap();
        assert!(contents.contains("actix_web::main"));
        assert!(!contents.contains("tokio::main"));
    }

    #[test]
    fn backend_build_rs_uses_env_var_not_cfg_macro() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/build.rs")).unwrap();
        assert!(contents.contains("CARGO_FEATURE_EMBED_ASSETS"));
        // cfg!() does not work for feature detection in build.rs
        assert!(!contents.contains("cfg!(feature"));
    }

    #[test]
    fn backend_cargo_toml_has_embed_assets_feature_with_include_dir() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("my-app/backend/Cargo.toml")).unwrap();
        assert!(contents.contains("embed-assets"));
        assert!(contents.contains("include_dir"));
    }

    #[test]
    fn shared_lib_does_not_use_deny_unknown_fields() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
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
        let result = scaffold(root.path(), "my-app");
        assert!(matches!(result, Err(InitError::AlreadyExists(_))));
    }

    #[test]
    fn project_name_substituted_into_crate_names() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "cool-project").unwrap();
        let contents =
            std::fs::read_to_string(root.path().join("cool-project/backend/Cargo.toml")).unwrap();
        assert!(contents.contains("cool-project"));
    }
}
