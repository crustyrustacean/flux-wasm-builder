// src/lib.rs

// module declarations
pub mod build;
pub mod env_check;
pub mod init;

// Re-exports for convenience
pub use build::{
    BuildConfig, BuildError, run_build_loop, run_command_with_timeout, run_wasm_pack,
    run_wasm_pack_with_env, start_watcher, FileWatcher,
};
pub use env_check::EnvCheckError;
pub use init::{scaffold, InitError};
