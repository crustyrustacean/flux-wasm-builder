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

/// Backend flux.toml
pub fn backend_flux_toml(name: &str) -> String {
    include_str!("templates/backend/flux.toml").replace("{{name}}", name)
}

/// Backend build.rs
pub fn backend_build_rs() -> &'static str {
    include_str!("templates/backend/build.rs")
}

/// Backend main.rs
pub fn backend_main_rs() -> &'static str {
    include_str!("templates/backend/main.rs")
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

/// Frontend screen.scss
pub fn frontend_screen_scss() -> &'static str {
    include_str!("templates/frontend/styles/screen.scss")
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
