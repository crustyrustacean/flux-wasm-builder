// src/domain.rs

// dependencies
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FluxConfig {
    pub project: ProjectConfig,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}