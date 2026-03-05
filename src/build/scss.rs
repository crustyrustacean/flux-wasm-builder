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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn compile_scss_produces_non_empty_css() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("screen.scss"), "body { color: red; }").unwrap();
        let result = compile_scss(dir.path()).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn compile_scss_output_is_valid_utf8() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("screen.scss"), "h1 { font-size: 2rem; }").unwrap();
        let result = compile_scss(dir.path()).unwrap();
        assert!(std::str::from_utf8(&result).is_ok());
    }

    #[test]
    fn compile_scss_expands_nesting() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("screen.scss"),
            ".parent { color: red; .child { color: blue; } }",
        )
        .unwrap();
        let result = compile_scss(dir.path()).unwrap();
        let css = std::str::from_utf8(&result).unwrap();
        assert!(
            css.contains(".parent .child"),
            "nested SCSS should be expanded"
        );
    }

    #[test]
    fn compile_scss_fails_for_missing_file() {
        let dir = tempdir().unwrap();
        // No screen.scss written
        let result = compile_scss(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn compile_scss_fails_for_invalid_scss() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("screen.scss"),
            "this is not valid scss }{}{",
        )
        .unwrap();
        let result = compile_scss(dir.path());
        assert!(matches!(result, Err(ScssError::CompileFailed(_))));
    }
}
