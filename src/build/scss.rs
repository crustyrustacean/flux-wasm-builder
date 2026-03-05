// src/build/scss.rs

// dependencies
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScssError {
    #[error("failed to compile SCSS: {0}")]
    CompileFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn compile_scss(styles_path: &Path) -> Result<Vec<u8>, ScssError> {
    let entry = styles_path.join("screen.scss");
    let css = grass::from_path(&entry, &grass::Options::default())
        .map_err(|e| ScssError::CompileFailed(e.to_string()))?;
    Ok(css.into_bytes())
}
