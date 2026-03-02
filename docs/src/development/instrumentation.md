# Instrumentation

flux-wasm-builder uses structured logging via `tracing` for comprehensive observability.

## Guiding Principles

### Errors are Data

Every error condition has a typed variant. This enables:

- Concrete `match` patterns in tests
- Clear error messages for operators
- Structured error handling

```rust
// Typed errors, not anyhow::Error
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("wasm-pack failed (exit code: {exit_code:?})")]
    WasmPackFailed { exit_code: Option<i32> },
    // ...
}
```

### Spans, Not Scattered Log Lines

`tracing` spans create a causal tree mapping to the tool's phases:

```rust
let span = tracing::info_span!("wasm_pack_build", frontend = %path.display());
let _enter = span.enter();

// All logs inside this span carry the frontend path automatically
tracing::info!("starting build");
```

### Operator-Legible Messages

Every error answers:

1. **What happened**
2. **Where it happened**
3. **What to do next**

```
# Bad
error: could not read file

# Good
error: frontend/index.html not found — has wasm-pack been run?
```

### No Silent Failures

Intentionally discarded results are documented:

```rust
// The `let _ =` is intentional: send returns Err when there
// are no active subscribers, which is expected when no browser
// tabs are open. This must not be treated as an error.
let _ = reload_tx.send(());
```

## Logging Configuration

### Initialization

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
    )
    .with_target(false)
    .compact()
    .init();
```

### Log Levels

| Level | Usage |
|-------|-------|
| `error` | Failures that stop operation |
| `warn` | Recoverable issues, build failures |
| `info` | Major lifecycle events |
| `debug` | Detailed operation info |
| `trace` | Very detailed diagnostics |

### Environment Control

```bash
# Default (info level)
cargo backend

# Show all debug messages
RUST_LOG=debug cargo backend

# Show only errors
RUST_LOG=error cargo backend

# Specific module
RUST_LOG=build_subsystem::watcher=debug cargo backend

# Multiple modules
RUST_LOG=build_subsystem=debug,actix_web=info cargo backend
```

## Span Structure

### Module Spans

Each major component has its own span:

```rust
// Build module
let span = tracing::info_span!(
    "wasm_pack_build",
    frontend = %config.frontend_crate_path.display(),
    timeout_secs = config.build_timeout_secs,
);

// Watcher module
let span = tracing::info_span!(
    "file_watcher",
    path = %watch_path.display(),
    debounce_ms
);

// WebSocket
let span = tracing::debug_span!("ws_connection", peer = %peer);
```

### Lifecycle Events

```rust
// Startup
tracing::info!("starting wasm-pack build");

// Progress
tracing::debug!(event_count = events.len(), "Rust source change detected");

// Completion
tracing::info!("rebuild succeeded");

// Errors
tracing::warn!(error = %e, "rebuild failed — previous assets still served");
```

## Error Logging

### Build Errors

```rust
match result {
    Ok(()) => {
        tracing::info!("rebuild succeeded");
    }
    Err(ref e) => {
        tracing::warn!(
            error = %e,
            "rebuild failed — previous assets still served"
        );
    }
}
```

### Watcher Errors

```rust
Err(e) => {
    // Non-fatal — log and continue
    tracing::warn!(error = %e, "file watcher error");
}
```

### WebSocket Errors

```rust
Err(broadcast::error::RecvError::Lagged(n)) => {
    tracing::warn!(
        skipped = n,
        "WebSocket subscriber lagged — sending one reload"
    );
}
```

## Structured Fields

Common fields used throughout:

| Field | Type | Description |
|-------|------|-------------|
| `error` | `Display` | Error message |
| `path` | `Display` | File path |
| `frontend` | `Display` | Frontend crate path |
| `timeout_secs` | `u64` | Timeout duration |
| `event_count` | `usize` | Number of events |
| `peer` | `String` | Client address |

## Log Output Examples

### Successful Build

```
INFO wasm_pack_build{frontend="../frontend" timeout_secs=300}: starting wasm-pack build
INFO wasm_pack_build{frontend="../frontend" timeout_secs=300}: wasm-pack build completed successfully
INFO actix_web::server: Actix-web server running at http://localhost:8080
INFO file_watcher{path="../frontend/src" debounce_ms=300}: file watcher active
```

### Build Failure

```
INFO wasm_pack_build{frontend="../frontend" timeout_secs=300}: starting wasm-pack build
WARN wasm_pack_build{frontend="../frontend" timeout_secs=300}: rebuild failed — previous assets still served error="wasm-pack failed (exit code: Some(1))"
```

### File Change

```
DEBUG file_watcher{path="../frontend/src" debounce_ms=300}: Rust source change detected event_count=1
INFO rebuild_cycle: rebuild succeeded
```

### WebSocket Connection

```
DEBUG ws_connection{peer="127.0.0.1:54321}: WebSocket connection established
DEBUG ws_connection{peer="127.0.0.1:54321}: sending reload signal to browser
```

## Testing with Logs

### Capture Logs in Tests

```rust
#[test]
fn test_with_logging() {
    // Initialize for test
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    
    // Test code here
}
```

### Assert on Log Output

For integration tests, capture stderr:

```rust
use assert_cmd::Command;

let output = Command::cargo_bin("flux-wasm-builder")
    .args(["init", "test"])
    .output()
    .unwrap();

let stderr = String::from_utf8_lossy(&output.stderr);
assert!(stderr.contains("starting"));
```

## Performance Considerations

### Avoid Logging in Hot Paths

```rust
// Bad: Logs on every request
tracing::debug!("handling request");

// Good: Log only interesting events
if let Err(e) = result {
    tracing::warn!(error = %e, "request failed");
}
```

### Use Appropriate Levels

```rust
// Trace for very detailed info
tracing::trace!("ignored non-.rs file event");

// Debug for normal operation
tracing::debug!("asset not found");

// Info for major events
tracing::info!("server started");
```

## Production Logging

For production deployments:

```bash
# Minimal logging
RUST_LOG=warn ./my-app-backend

# Structured JSON output (requires tracing-subscriber json feature)
RUST_LOG=info ./my-app-backend 2>&1 | jq
```
