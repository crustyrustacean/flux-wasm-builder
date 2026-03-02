//! Template contents for generated project files.

/// Workspace root Cargo.toml
pub fn workspace_cargo_toml(name: &str) -> String {
    format!(
        r#"[workspace]
resolver = "3"
members = ["backend", "frontend", "shared"]

[workspace.package]
name = "{name}"
version = "0.1.0"
edition = "2024"
"#
    )
}

/// Cargo config with alias for backend
pub fn cargo_config(name: &str) -> String {
    // Use the full package name for the alias
    format!(
        r#"[alias]
backend = "run -p {name}-backend"
start = "run -p {name}-backend"
"#
    )
}

/// .gitignore for the workspace
pub fn gitignore() -> &'static str {
    r#"target/
frontend/pkg/
"#
}

/// Backend Cargo.toml
pub fn backend_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}-backend"
version.workspace = true
edition.workspace = true

[dependencies]
actix-web = "4"
actix-ws = "0.3"
tokio = {{ version = "1", features = ["full"] }}
serde_json = "1"
mime_guess = "2"
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
notify-debouncer-mini = "0.4"
thiserror = "1"
{name}-shared = {{ path = "../shared" }}

[features]
embed-assets = ["dep:include_dir"]

[dependencies.include_dir]
version = "0.7"
optional = true
"#
    )
}

/// Backend build.rs
pub fn backend_build_rs() -> &'static str {
    r#"fn main() {
    // Use the environment variable, not cfg!(), for feature detection in build.rs
    if std::env::var("CARGO_FEATURE_EMBED_ASSETS").is_ok() {
        let pkg = std::path::Path::new("../frontend/pkg");
        if !pkg.exists() || pkg.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            panic!(
                "\n\nembedding assets requires frontend/pkg/ to exist and be non-empty.\n\
                 Run this first:\n\n  \
                 wasm-pack build frontend/ --target web --release\n\n"
            );
        }
    }
}
"#
}

/// Backend main.rs
pub fn backend_main_rs() -> &'static str {
    r#"use actix_web::{web, App, HttpServer};
use tracing_subscriber::{fmt, EnvFilter};

mod build_subsystem;
mod api;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let port = 8080;
    
    // In dev mode, the build subsystem would start here
    // For now, this is a stub that will be implemented in Stage 1
    
    let server = HttpServer::new(move || {
        App::new()
            .configure(api::configure)
            // Static assets and SPA fallback will be added in Stage 2
    })
    .bind(("127.0.0.1", port));

    match server {
        Ok(s) => {
            tracing::info!(port, "server listening");
            s.run().await?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("error: port {port} is already in use. Change the port in BuildConfig.");
            std::process::exit(1);
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

fn init_tracing() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_target(false)
        .compact()
        .init();
}
"#
}

/// Backend build_subsystem/mod.rs
pub fn backend_build_subsystem_mod() -> &'static str {
    r#"pub mod build;
pub mod build_coordinator;
pub mod watcher;
pub mod reload;
pub mod static_assets;
"#
}

/// Stub file for unimplemented modules
pub fn stub_file() -> &'static str {
    "// Implementation will be added in a future stage\n"
}

/// Backend API module
pub fn backend_api_mod(name: &str) -> String {
    // Convert hyphens to underscores for valid Rust identifier
    let crate_name = name.replace('-', "_");
    format!(
        r#"use actix_web::{{web, HttpResponse, Responder}};
use serde_json::json;
use {crate_name}_shared::HelloResponse;

pub fn configure(cfg: &mut web::ServiceConfig) {{
    cfg.service(
        web::scope("/api")
            .route("/hello", web::get().to(hello))
            .route("/status", web::get().to(status))
    );
}}

async fn hello() -> impl Responder {{
    HttpResponse::Ok().json(HelloResponse {{
        message: "Hello from the backend!".to_string(),
    }})
}}

async fn status() -> impl Responder {{
    HttpResponse::Ok().json(json!({{
        "version": env!("CARGO_PKG_VERSION"),
        "status": "ok"
    }}))
}}
"#
    )
}

/// Frontend Cargo.toml
pub fn frontend_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}-frontend"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
yew = {{ version = "0.22.1", features = ["csr"] }}
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
wasm-logger = "0.2.0"
web-sys = {{ version = "0.3", features = [ "Window", "Document" ] }}
gloo-net = "0.5"
log = "0.4.29"
console_error_panic_hook = "0.1.7"
{name}-shared = {{ path = "../shared" }}
"#
    )
}

/// Frontend index.html
pub fn frontend_index_html(name: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{name}</title>
</head>
<body>
    <script type="module">
        import init from './pkg/{name}_frontend.js';
        init();
    </script>
</body>
</html>
"#
    )
}

/// Frontend lib.rs
pub fn frontend_lib_rs(name: &str) -> String {
    // Convert hyphens to underscores for valid Rust identifier
    let crate_name = name.replace('-', "_");
    format!(
        r#"use yew::prelude::*;
use gloo_net::http::Request;
use {crate_name}_shared::HelloResponse;

#[component(App)]
fn app() -> Html {{
    let message = use_state(|| None::<String>);

    {{
        let message = message.clone();
        use_effect_with((), move |_| {{
            let message = message.clone();
            wasm_bindgen_futures::spawn_local(async move {{
                match Request::get("/api/hello")
                    .send()
                    .await
                {{
                    Ok(response) => {{
                        if let Ok(hello) = response.json::<HelloResponse>().await {{
                            message.set(Some(hello.message));
                        }}
                    }}
                    Err(e) => {{
                        web_sys::console::log_1(&format!("Error: {{:?}}", e).into());
                    }}
                }}
            }});
            || ()
        }});
    }}

    html! {{
        <div>
            <h1>{{ "Flux WASM Builder" }}</h1>
            {{
                if let Some(msg) = (*message).clone() {{
                    html! {{ <p>{{ msg }}</p> }}
                }} else {{
                    html! {{ <p>{{ "Loading..." }}</p> }}
                }}
            }}
        </div>
    }}
}}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {{
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}}
"#
    )
}

/// Shared Cargo.toml
pub fn shared_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}-shared"
version.workspace = true
edition.workspace = true

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
"#
    )
}

/// Shared lib.rs
pub fn shared_lib_rs() -> &'static str {
    r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
}
"#
}
