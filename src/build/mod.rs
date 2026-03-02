// src/build/mod.rs

//! Build subsystem for invoking wasm-pack.
//!
//! This module provides the core functionality for building the frontend
//! WebAssembly crate using `wasm-pack`. It handles:
//!
//! - Spawning the wasm-pack subprocess
//! - Streaming stdout/stderr to the parent process in real time
//! - Timeout handling with process termination
//! - Typed error reporting

mod build;

pub use build::{BuildConfig, BuildError, run_wasm_pack, run_wasm_pack_with_env, run_command_with_timeout};
