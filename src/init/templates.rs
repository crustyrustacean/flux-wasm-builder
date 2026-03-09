//! Template rendering for generated project files.
//!
//! All templates are embedded at compile time using `include_str!` and
//! rendered via Tera. The public API is a single `render_template` function
//! that takes a template path and a `ProjectContext`.
//!
//! ## Template Paths
//! Template paths mirror the file structure under `src/init/templates/`,
//! e.g. `"workspace/cargo.toml"` or `"backend/src/lib.rs"`.
//!
//! ## Adding New Variables
//! Add the field to `ProjectContext`, populate it in `ProjectContext::new()`,
//! and add it to the `From<&ProjectContext> for tera::Context` impl.
//! No other changes required.

use std::sync::LazyLock;
use tera::Tera;

/// Error type for template rendering failures.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("template render failed for '{template}': {source}")]
    RenderFailed {
        template: String,
        source: tera::Error,
    },
}

/// All variables available to every template.
///
/// Built once from the project name and passed to every `render_template` call.
pub struct ProjectContext {
    /// The project name as given by the user (e.g. "my-app")
    pub name: String,
    /// The project name with hyphens replaced by underscores (e.g. "my_app")
    /// Used in Rust crate names and module paths.
    pub crate_name: String,
    /// The project name with hyphens replaced by underscores (e.g. "my_app")
    /// Used in JavaScript import paths.
    pub js_name: String,
}

impl ProjectContext {
    pub fn new(name: &str) -> Self {
        let crate_name = name.replace('-', "_");
        let js_name = crate_name.clone();
        Self {
            name: name.to_string(),
            crate_name,
            js_name,
        }
    }
}

impl From<&ProjectContext> for tera::Context {
    fn from(ctx: &ProjectContext) -> Self {
        let mut tera_ctx = tera::Context::new();
        tera_ctx.insert("name", &ctx.name);
        tera_ctx.insert("crate_name", &ctx.crate_name);
        tera_ctx.insert("js_name", &ctx.js_name);
        tera_ctx
    }
}

/// The global Tera instance with all templates embedded at compile time.
static TERA: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.autoescape_on(vec![]);

    tera.add_raw_templates(vec![
        // Workspace
        (
            "workspace/cargo.toml",
            include_str!("templates/workspace/cargo.toml"),
        ),
        (
            "workspace/cargo-config.toml",
            include_str!("templates/workspace/cargo-config.toml"),
        ),
        ("gitignore", include_str!("templates/gitignore")),
        // Backend
        (
            "backend/cargo.toml",
            include_str!("templates/backend/cargo.toml"),
        ),
        (
            "backend/drydock.toml",
            include_str!("templates/backend/drydock.toml"),
        ),
        (
            "backend/build.rs",
            include_str!("templates/backend/build.rs"),
        ),
        (
            "backend/src/bin/main.rs",
            include_str!("templates/backend/src/bin/main.rs"),
        ),
        (
            "backend/src/lib.rs",
            include_str!("templates/backend/src/lib.rs"),
        ),
        (
            "backend/src/configuration.rs",
            include_str!("templates/backend/src/configuration.rs"),
        ),
        (
            "backend/src/error.rs",
            include_str!("templates/backend/src/error.rs"),
        ),
        (
            "backend/src/response.rs",
            include_str!("templates/backend/src/response.rs"),
        ),
        (
            "backend/src/telemetry.rs",
            include_str!("templates/backend/src/telemetry.rs"),
        ),
        (
            "backend/src/startup.rs",
            include_str!("templates/backend/src/startup.rs"),
        ),
        (
            "backend/src/static_assets.rs",
            include_str!("templates/backend/src/static_assets.rs"),
        ),
        (
            "backend/api/mod.rs",
            include_str!("templates/backend/api/mod.rs"),
        ),
        (
            "backend/configuration/base.yaml",
            include_str!("templates/backend/configuration/base.yaml"),
        ),
        (
            "backend/configuration/local.yaml",
            include_str!("templates/backend/configuration/local.yaml"),
        ),
        (
            "backend/configuration/production.yaml",
            include_str!("templates/backend/configuration/production.yaml"),
        ),
        (
            "backend/tests/api/main.rs",
            include_str!("templates/backend/tests/api/main.rs"),
        ),
        (
            "backend/tests/api/helpers.rs",
            include_str!("templates/backend/tests/api/helpers.rs"),
        ),
        (
            "backend/tests/api/health_check.rs",
            include_str!("templates/backend/tests/api/health_check.rs"),
        ),
        // Frontend
        (
            "frontend/cargo.toml",
            include_str!("templates/frontend/cargo.toml"),
        ),
        (
            "frontend/index.html",
            include_str!("templates/frontend/index.html"),
        ),
        ("frontend/lib.rs", include_str!("templates/frontend/lib.rs")),
        (
            "frontend/styles/screen.scss",
            include_str!("templates/frontend/styles/screen.scss"),
        ),
        // Shared
        (
            "shared/cargo.toml",
            include_str!("templates/shared/cargo.toml"),
        ),
        ("shared/lib.rs", include_str!("templates/shared/lib.rs")),
        // Deployment
        (
            "deployment/Dockerfile",
            include_str!("templates/deployment/Dockerfile"),
        ),
        (
            "deployment/dockerignore",
            include_str!("templates/deployment/dockerignore"),
        ),
        (
            "deployment/fly.toml",
            include_str!("templates/deployment/fly.toml"),
        ),
    ])
    .expect("failed to load embedded templates — this is a bug in wasm-drydock");

    tera
});

/// Render a template by path with the given project context.
///
/// # Arguments
/// * `template_path` - Path matching the key used in `TERA`, e.g. `"backend/src/lib.rs"`
/// * `ctx` - The project context containing all template variables
///
/// # Errors
/// Returns `TemplateError::RenderFailed` if Tera cannot render the template.
pub fn render_template(template_path: &str, ctx: &ProjectContext) -> Result<String, TemplateError> {
    let tera_ctx = tera::Context::from(ctx);
    TERA.render(template_path, &tera_ctx)
        .map_err(|e| TemplateError::RenderFailed {
            template: template_path.to_string(),
            source: e,
        })
}
