//! wasm-drydock CLI entry point
//!
//! This binary provides three commands for managing fullstack Rust web applications:
//!
//! - `init` - Scaffold a new three-crate workspace project
//! - `dev` - Start the development server with hot-reloading
//! - `release` - Build an optimized, self-contained production binary

// dependencies
use clap::{Parser, Subcommand};
use wasm_drydock::dev;
use wasm_drydock::init::scaffold;
use wasm_drydock::release;
use std::path::PathBuf;

/// CLI parser for wasm-drydock
#[derive(Parser)]
#[command(name = "wasm-drydock")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available CLI commands
#[derive(Subcommand)]
enum Commands {
    /// Create a new fullstack project
    ///
    /// Scaffolds a three-crate Cargo workspace with:
    /// - `backend/` - Actix-web API server
    /// - `frontend/` - Yew WASM application
    /// - `shared/` - Serde-compatible API types
    Init {
        /// Project name (will be used as directory name)
        name: String,
        /// Parent directory (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },

    /// Launch the development server to build the project
    ///
    /// Starts a dev server that:
    /// - Builds the frontend with `wasm-pack` on startup
    /// - Spawns the Actix-web backend and waits for it to be ready
    /// - Proxies `/api/*` requests to the backend
    /// - Watches source files and rebuilds/restarts on changes
    /// - Signals the browser to reload after successful builds
    Dev {
        /// Open browser automatically on server start
        ///
        /// When set, opens the default browser at `http://127.0.0.1:{public_port}`
        /// (default port: 8080) after the server is ready.
        #[arg(short, long)]
        open: bool,
    },

    /// Build a release binary of the final project
    ///
    /// Runs `wasm-pack build --release` on the frontend, then
    /// `cargo build --release --features embed-assets` on the backend.
    /// The result is a single self-contained binary with all frontend
    /// assets and base configuration compiled in.
    Release,
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init { name, path } => {
            scaffold(&path, &name)?;
        }

        Commands::Dev { open } => {
            dev::run(open).await?;
        }

        Commands::Release => {
            release::run().await?;
        }
    }
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
