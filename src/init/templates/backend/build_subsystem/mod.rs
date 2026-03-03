pub mod build;
pub mod build_coordinator;
pub mod watcher;
pub mod static_assets;

// Dev-mode only module
#[cfg(not(feature = "embed-assets"))]
pub mod reload;

/// Flag indicating development mode.
/// When true, the reload script is injected into index.html.
#[derive(Clone, Copy, Debug)]
pub struct DevMode(pub bool);
