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
//!
//! # Feature Flags
//!
//! - `embed-assets`: Embeds frontend assets into the binary at compile time.
//!   When this feature is active, the reload module and file watcher are
//!   compiled out, and assets are served from embedded data.

mod build_coordinator;
pub mod scss;
mod static_assets;
pub mod wasm_pack;
mod watcher;

// Reload module is only needed in dev mode (not embed-assets)
#[cfg(not(feature = "embed-assets"))]
mod reload;

pub use build_coordinator::run_build_loop;
pub use static_assets::{serve_pkg_file, spa_fallback};
pub use wasm_pack::{
    BuildConfig, BuildError, run_command_with_timeout, run_wasm_pack, run_wasm_pack_with_env,
};
pub use watcher::{FileWatcher, start_watcher};

// Only export reload functionality in dev mode
#[cfg(not(feature = "embed-assets"))]
pub use reload::{RELOAD_SCRIPT, inject_reload_script, ws_reload_handler};

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
