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
//! - WebSocket-based live reload for development

mod build;
mod build_coordinator;
mod reload;
mod static_assets;
mod watcher;

pub use build::{BuildConfig, BuildError, run_wasm_pack, run_wasm_pack_with_env, run_command_with_timeout};
pub use build_coordinator::run_build_loop;
pub use reload::{inject_reload_script, ws_reload_handler, RELOAD_SCRIPT};
pub use static_assets::{serve_pkg_file, spa_fallback};
pub use watcher::{start_watcher, FileWatcher};

/// Flag indicating whether the server is running in development mode.
///
/// In dev mode:
/// - Reload script is injected into index.html
/// - WebSocket endpoint is available at /ws/reload
/// - File watcher monitors frontend source
///
/// In release mode (embed-assets feature):
/// - No reload script injection
/// - WebSocket endpoint is compiled out
/// - Assets are embedded in the binary
#[derive(Clone, Copy, Debug)]
pub struct DevMode(pub bool);
