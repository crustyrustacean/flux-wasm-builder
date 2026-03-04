// tests/domain.rs
//
// Integration tests for domain configuration.
// Run with: cargo test --test domain

use flux_wasm_builder::domain::FluxConfig;
use tempfile::tempdir;

#[test]
fn flux_config_parses_complete_config() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("flux.toml");
    std::fs::write(
        &config_path,
        r#"
[project]
name = "my-app"

[dev]
public_port = 3000
backend_port = 4000
watch_debounce_ms = 500
"#,
    )
    .unwrap();

    let config = FluxConfig::from_file(&config_path).unwrap();
    assert_eq!(config.project.name, "my-app");
    assert_eq!(config.dev.public_port, 3000);
    assert_eq!(config.dev.backend_port, 4000);
    assert_eq!(config.dev.watch_debounce_ms, 500);
}

#[test]
fn flux_config_missing_dev_section_uses_defaults() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("flux.toml");
    std::fs::write(&config_path, "[project]\nname = \"test\"").unwrap();

    let config = FluxConfig::from_file(&config_path).unwrap();
    assert_eq!(config.dev.public_port, 8080);
    assert_eq!(config.dev.backend_port, 3001);
    assert_eq!(config.dev.watch_debounce_ms, 300);
}
