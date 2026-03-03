//! Template contents for generated project files.
//!
//! Templates are stored as separate files in the `templates/` directory
//! and loaded at compile time using `include_str!`.
//!
//! ## Static Templates
//! Templates with no variable substitution return `&'static str` directly.
//!
//! ## Dynamic Templates
//! Templates requiring name substitution use `{{name}}` placeholders that
//! are replaced at runtime. The following placeholders are supported:
//! - `{{name}}` - The project name (e.g., "my-app")
//! - `{{crate_name}}` - The project name with hyphens replaced by underscores (e.g., "my_app")
//! - `{{js_name}}` - The project name with hyphens replaced by underscores for JS imports

// =============================================================================
// Workspace Templates
// =============================================================================

/// Workspace root Cargo.toml
pub fn workspace_cargo_toml(name: &str) -> String {
    include_str!("templates/workspace/cargo.toml").replace("{{name}}", name)
}

/// Cargo config with alias for backend
pub fn cargo_config(name: &str) -> String {
    include_str!("templates/workspace/cargo-config.toml").replace("{{name}}", name)
}

/// .gitignore for the workspace
pub fn gitignore() -> &'static str {
    include_str!("templates/gitignore")
}

// =============================================================================
// Backend Templates
// =============================================================================

/// Backend Cargo.toml
pub fn backend_cargo_toml(name: &str) -> String {
    include_str!("templates/backend/cargo.toml").replace("{{name}}", name)
}

/// Backend build.rs
pub fn backend_build_rs() -> &'static str {
    include_str!("templates/backend/build.rs")
}

/// Backend main.rs
pub fn backend_main_rs() -> &'static str {
    include_str!("templates/backend/main.rs")
}

/// Backend build_subsystem/mod.rs
pub fn backend_build_subsystem_mod() -> &'static str {
    include_str!("templates/backend/build_subsystem/mod.rs")
}

/// Backend build_subsystem/build.rs - wasm-pack invocation
pub fn backend_build_subsystem_build_rs() -> &'static str {
    include_str!("templates/backend/build_subsystem/build.rs")
}

/// Backend build_subsystem/build_coordinator.rs - build loop with coalescing
pub fn backend_build_subsystem_build_coordinator_rs() -> &'static str {
    include_str!("templates/backend/build_subsystem/build_coordinator.rs")
}

/// Backend build_subsystem/watcher.rs - file watcher for .rs source changes
pub fn backend_build_subsystem_watcher_rs() -> &'static str {
    include_str!("templates/backend/build_subsystem/watcher.rs")
}

/// Backend build_subsystem/static_assets.rs - asset serving handlers
pub fn backend_build_subsystem_static_assets_rs() -> &'static str {
    include_str!("templates/backend/build_subsystem/static_assets.rs")
}

/// Backend build_subsystem/reload.rs - WebSocket live reload
pub fn backend_build_subsystem_reload_rs() -> &'static str {
    include_str!("templates/backend/build_subsystem/reload.rs")
}

/// Backend API module
pub fn backend_api_mod(name: &str) -> String {
    let crate_name = name.replace('-', "_");
    include_str!("templates/backend/api/mod.rs").replace("{{crate_name}}", &crate_name)
}

// =============================================================================
// Frontend Templates
// =============================================================================

/// Frontend Cargo.toml
pub fn frontend_cargo_toml(name: &str) -> String {
    include_str!("templates/frontend/cargo.toml").replace("{{name}}", name)
}

/// Frontend index.html
pub fn frontend_index_html(name: &str) -> String {
    let js_name = name.replace('-', "_");
    include_str!("templates/frontend/index.html")
        .replace("{{name}}", name)
        .replace("{{js_name}}", &js_name)
}

/// Frontend lib.rs
pub fn frontend_lib_rs(name: &str) -> String {
    let crate_name = name.replace('-', "_");
    include_str!("templates/frontend/lib.rs").replace("{{crate_name}}", &crate_name)
}

// =============================================================================
// Shared Templates
// =============================================================================

/// Shared Cargo.toml
pub fn shared_cargo_toml(name: &str) -> String {
    include_str!("templates/shared/cargo.toml").replace("{{name}}", name)
}

/// Shared lib.rs
pub fn shared_lib_rs() -> &'static str {
    include_str!("templates/shared/lib.rs")
}
