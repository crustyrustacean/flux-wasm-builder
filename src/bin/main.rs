// src/main.rs

// dependencies
use clap::{Parser, Subcommand};
use flux_wasm_builder::init::scaffold;
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
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init { name, path } => {
            scaffold(&path, &name)?;
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

fn main() {
    init_tracing();

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
