// src/lib.rs

//! wasm-drydock - A single-command dev tool for fullstack Rust web applications.
//!
//! This crate provides the core functionality for scaffolding and developing
//! fullstack Rust applications using Actix-web and Yew.

// module declarations
pub mod build;
pub mod dev;
pub mod domain;
pub mod env_check;
pub mod init;
pub mod release;
pub mod validation;

// Re-exports for convenience
pub use build::{
    BuildConfig, BuildError, BuildMode, DevMode, DevServerMessage, FileWatcher, run_build_loop,
    run_command_with_timeout, run_wasm_pack, run_wasm_pack_with_env, serve_pkg_file, spa_fallback,
    start_watcher,
};

// Only export reload functionality in dev mode (not embed-assets)
#[cfg(not(feature = "embed-assets"))]
pub use build::{RELOAD_SCRIPT, inject_reload_script, ws_reload_handler};

pub use dev::run;
pub use env_check::EnvCheckError;
pub use init::{InitError, scaffold};
pub use validation::ValidationError;
