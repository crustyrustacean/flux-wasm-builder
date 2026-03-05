// src/main.rs

// dependencies
use clap::{Parser, Subcommand};
use flux_wasm_builder::dev;
use flux_wasm_builder::init::scaffold;
use flux_wasm_builder::release;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "flux-wasm-builder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new fullstack project
    Init {
        /// Project name (will be used as directory name)
        name: String,
        /// Parent directory (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },

    /// Launch the development server to build the project
    Dev,

    /// Build a release binary of the final project
    Release,
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init { name, path } => {
            scaffold(&path, &name)?;
        }

        Commands::Dev => {
            dev::run().await?;
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
