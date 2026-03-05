// src/lib.rs

pub mod api;
pub mod configuration;
pub mod error;
pub mod response;
pub mod startup;
pub mod telemetry;

#[cfg(feature = "embed-assets")]
pub mod static_assets;
