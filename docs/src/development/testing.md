# Testing

flux-wasm-builder has comprehensive test coverage across unit and integration tests.

## Running Tests

### Standard Tests

```bash
cargo test
```

### Tests Requiring wasm-pack

Some tests require wasm-pack to be installed:

```bash
RUN_WASM_TESTS=1 cargo test
```

Without this flag, wasm-pack tests are skipped.

### Verbose Output

```bash
cargo test -- --nocapture
```

### Specific Tests

```bash
# Run specific test
cargo test test_name

# Run tests in specific file
cargo test --test init

# Run tests matching pattern
cargo test watcher
```

## Test Organization

### Unit Tests

Located in source files under `#[cfg(test)] mod tests`:

```rust
// src/build/watcher.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn rs_file_change_triggers_build_signal() {
        // ...
    }
}
```

### Integration Tests

Located in `tests/` directory:

| File | Coverage |
|------|----------|
| `tests/init.rs` | Project scaffolding |
| `tests/build.rs` | Build subsystem |
| `tests/serve.rs` | Static asset serving |
| `tests/watcher.rs` | File watching and coordination |
| `tests/reload.rs` | Live reload functionality |
| `tests/api.rs` | Shared types and API handlers |

## Test Categories

### Environment Tests

Test environment validation:

```rust
#[test]
fn version_meets_minimum_returns_true_for_equal_versions() {
    assert!(version_meets_minimum("0.13.0", "0.13.0"));
}

#[test]
fn version_meets_minimum_returns_false_for_older_versions() {
    assert!(!version_meets_minimum("0.12.0", "0.13.0"));
}
```

### Scaffolding Tests

Test project generation:

```rust
#[test]
fn init_creates_project_in_current_directory() {
    let dir = tempdir().unwrap();
    cargo_bin_cmd!("flux-wasm-builder")
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success();
    
    assert!(dir.path().join("test-project/Cargo.toml").exists());
}
```

### Build Tests

Test wasm-pack invocation:

```rust
#[tokio::test]
async fn build_timeout_kills_process() {
    let config = BuildConfig {
        build_timeout_secs: 1,
        ..BuildConfig::new("/nonexistent")
    };
    
    let result = run_wasm_pack(&config).await;
    assert!(matches!(result, Err(BuildError::Timeout { .. })));
}
```

### Watcher Tests

Test file watching:

```rust
#[test]
fn rs_file_change_triggers_build_signal() {
    let dir = tempdir().unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    
    let _watcher = start_watcher(dir.path(), 50, tx).unwrap();
    std::fs::write(dir.path().join("lib.rs"), b"modified").unwrap();
    
    assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok());
}

#[test]
fn non_rs_file_does_not_trigger_build_signal() {
    // Should timeout - no signal expected
}
```

### Build Coordinator Tests

Test rebuild coordination:

```rust
#[tokio::test]
async fn rapid_triggers_collapse_to_at_most_two_builds() {
    let (tx, rx) = mpsc::channel(8);
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let count = Arc::new(Mutex::new(0u32));
    
    // Send 5 rapid triggers
    for _ in 0..5 {
        tx.send(()).await.unwrap();
    }
    drop(tx);
    
    run_build_loop(rx, reload_tx, /* ... */ ).await;
    
    assert!(*count.lock().unwrap() <= 2);
}
```

### HTTP Handler Tests

Test Actix-web handlers:

```rust
#[actix_web::test]
async fn serves_wasm_with_application_wasm_content_type() {
    let pkg_dir = tempdir().unwrap();
    std::fs::write(pkg_dir.path().join("app_bg.wasm"), b"\0asm").unwrap();
    
    let config = BuildConfig {
        pkg_output_path: pkg_dir.path().to_path_buf(),
        ..BuildConfig::new("/unused")
    };
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config))
            .route("/pkg/{filename}", web::get().to(serve_pkg_file))
    ).await;
    
    let req = test::TestRequest::get().uri("/pkg/app_bg.wasm").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), 200);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("application/wasm"));
}
```

### WebSocket Tests

Test live reload WebSocket:

```rust
#[actix_web::test]
async fn ws_endpoint_returns_101_switching_protocols() {
    let (tx, _) = broadcast::channel::<()>(16);
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(tx))
            .route("/ws/reload", web::get().to(ws_reload_handler))
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/ws/reload")
        .insert_header(("upgrade", "websocket"))
        .insert_header(("connection", "upgrade"))
        .insert_header(("sec-websocket-version", "13"))
        .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
        .to_request();
    
    assert_eq!(test::call_service(&app, req).await.status(), 101);
}
```

## Test Utilities

### tempfile

Create temporary directories:

```rust
use tempfile::tempdir;

let dir = tempdir().unwrap();
let path = dir.path();  // Cleaned up when dir is dropped
```

### assert_cmd

Test CLI commands:

```rust
use assert_cmd::cargo::cargo_bin_cmd;

cargo_bin_cmd!("flux-wasm-builder")
    .args(["init", "test-app"])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(contains("test-app"));
```

### predicates

Assert on output:

```rust
use predicates::str::contains;

.assert()
    .stdout(contains("expected output"))
    .stderr(contains("error message"));
```

## Test Coverage

Run tests with coverage:

```bash
cargo tarpaulin --out Html
```

Or using cargo-llvm-cov:

```bash
cargo llvm-cov --html
```

## Continuous Integration

Tests run automatically on:

- Every push to main
- Every pull request

CI configuration:

```yaml
# .github/workflows/test.yml
- name: Run tests
  run: cargo test
  
- name: Run wasm tests
  run: RUN_WASM_TESTS=1 cargo test
```

## Writing New Tests

### Guidelines

1. **Test behavior, not implementation**
2. **Use descriptive names**: `subject_condition_expected`
3. **Follow AAA pattern**: Arrange, Act, Assert
4. **Test edge cases**: Empty input, errors, boundaries
5. **Keep tests independent**: No shared state

### Example Template

```rust
#[test]
fn function_name_input_condition_expected_result() {
    // Arrange
    let input = setup_test_data();
    
    // Act
    let result = function_under_test(input);
    
    // Assert
    assert_eq!(result, expected);
    
    // Cleanup (if needed)
    // automatic with tempfile
}
```
