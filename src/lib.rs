// src/lib.rs

// module declarations
pub mod build;
pub mod env_check;
pub mod init;

// Re-exports for convenience
pub use build::{BuildConfig, BuildError};
pub use env_check::EnvCheckError;
pub use init::{scaffold, InitError};
