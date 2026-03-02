// src/build/mod.rs

//! Build subsystem for invoking wasm-pack and serving static assets.
//!
//! This module provides the core functionality for building the frontend
//! WebAssembly crate using `wasm-pack` and serving the resulting assets.
//! It handles:
//!
//! - Spawning the wasm-pack subprocess
//! - Streaming stdout/stderr to the parent process in real time
//! - Timeout handling with process termination
//! - Typed error reporting
//! - Static asset serving for pkg/ directory
//! - SPA fallback for client-side routing
//! - File watching for .rs source changes
//! - Build coordination with coalescing

mod build;
mod build_coordinator;
mod static_assets;
mod watcher;

pub use build::{BuildConfig, BuildError, run_wasm_pack, run_wasm_pack_with_env, run_command_with_timeout};
pub use build_coordinator::run_build_loop;
pub use static_assets::{serve_pkg_file, spa_fallback};
pub use watcher::{start_watcher, FileWatcher};
