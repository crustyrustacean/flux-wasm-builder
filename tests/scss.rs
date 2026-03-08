// tests/scss.rs

use wasm_drydock::build::scss::{ScssError, compile_scss};
use tempfile::tempdir;

#[test]
fn compile_scss_with_variables() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("screen.scss"),
        "$primary: #333; body { color: $primary; }",
    )
    .unwrap();
    let result = compile_scss(dir.path()).unwrap();
    let css = std::str::from_utf8(&result).unwrap();
    assert!(
        css.contains("#333") || css.contains("333"),
        "SCSS variable should be resolved"
    );
}

#[test]
fn compile_scss_with_imports() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("_base.scss"),
        "* { box-sizing: border-box; }",
    )
    .unwrap();
    std::fs::write(dir.path().join("screen.scss"), "@use 'base';").unwrap();
    let result = compile_scss(dir.path());
    // Should compile without error
    assert!(result.is_ok(), "SCSS with @use should compile successfully");
}

#[test]
fn compile_scss_returns_compile_failed_error_variant() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("screen.scss"), "$x: ;").unwrap(); // invalid
    let result = compile_scss(dir.path());
    assert!(matches!(result, Err(ScssError::CompileFailed(_))));
}
