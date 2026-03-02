# Build Subsystem

The build subsystem handles frontend compilation via `wasm-pack` subprocess invocation.

## Module Structure

| File | Purpose |
|------|---------|
| [`build.rs`](https://github.com/crustyrustacean/flux-wasm-builder/blob/main/src/build/build.rs) | wasm-pack invocation |
| [`build_coordinator.rs`](https://github.com/crustyrustacean/flux-wasm-builder/blob/main/src/build/build_coordinator.rs) | Serialized rebuild loop |

## wasm-pack Invocation

### Command Construction

```rust
let mut cmd = Command::new("wasm-pack");
cmd.args(["build", "--target", "web"])
    .arg(&config.frontend_crate_path)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());

// Dev mode flag
#[cfg(debug_assertions)]
cmd.arg("--dev");

// Release mode
#[cfg(not(debug_assertions))]
cmd.arg("--release");
```

### Output Streaming

stdout and stderr are inherited directly to the parent process:

- Rust compiler diagnostics appear inline
- No buffering delays
- Real-time feedback during builds

### Timeout Handling

Builds are wrapped with a configurable timeout:

```rust
let result = tokio::time::timeout(
    Duration::from_secs(config.build_timeout_secs),
    cmd.status(),
).await;
```

If the timeout is exceeded:

1. The child process is killed
2. `BuildError::Timeout` is returned
3. The server continues with previous assets

## Build Error Types

```rust
pub enum BuildError {
    /// wasm-pack exited with non-zero status
    WasmPackFailed { exit_code: Option<i32> },
    
    /// Build timed out and process was killed
    Timeout { secs: u64 },
    
    /// wasm-pack binary not found or couldn't spawn
    SpawnFailed { source: std::io::Error },
    
    /// Waiting on process failed
    WaitFailed { source: std::io::Error },
    
    /// Killing timed-out process failed
    KillFailed { source: std::io::Error },
}
```

## Build Coordinator

The build coordinator serializes rebuild requests and coalesces rapid triggers.

### Behavior

1. Wait for signal on `build_rx` channel
2. Drain any queued messages, set `pending` flag
3. Execute build function
4. On success, send reload signal via `reload_tx`
5. If `pending`, immediately start another build
6. Exit when channel closes

### Coalescing Logic

```rust
// Drain queued messages
while build_rx.try_recv().is_ok() {
    pending = true;
}

// At most two builds: one running + one pending
```

This ensures:

- Rapid file saves don't queue endless builds
- At most one extra build is queued
- Builds run sequentially, never concurrently

### Error Handling

Build failures are **not fatal** to the coordinator:

```rust
match result {
    Ok(()) => {
        let _ = reload_tx.send(());  // Ignore no-receivers error
    }
    Err(ref e) => {
        tracing::warn!("rebuild failed — previous assets still served");
    }
}
```

The server continues running with the last successful build.

## BuildConfig

```rust
pub struct BuildConfig {
    /// Path to frontend crate (e.g., "../frontend")
    pub frontend_crate_path: PathBuf,
    
    /// Path to pkg output (default: frontend_crate_path + "/pkg")
    pub pkg_output_path: PathBuf,
    
    /// Path to index.html (default: frontend_crate_path + "/index.html")
    pub index_html_path: PathBuf,
    
    /// Watch debounce in milliseconds (default: 300)
    pub watch_debounce_ms: u64,
    
    /// Server port (default: 8080)
    pub port: u16,
    
    /// Build timeout in seconds (default: 300)
    pub build_timeout_secs: u64,
}
```

### Creating Configuration

```rust
// With defaults
let config = BuildConfig::new("../frontend");

// Custom values
let config = BuildConfig {
    port: 3000,
    build_timeout_secs: 600,
    ..BuildConfig::new("../frontend")
};
```

## Testing

The build subsystem is designed for testability:

```rust
#[tokio::test]
async fn build_timeout_kills_process() {
    let config = BuildConfig {
        build_timeout_secs: 1,
        ..BuildConfig::new("../frontend")
    };
    
    let result = run_wasm_pack(&config).await;
    assert!(matches!(result, Err(BuildError::Timeout { .. })));
}
```

### Test Helpers

```rust
/// Run wasm-pack with custom environment (for testing)
pub async fn run_wasm_pack_with_env(
    config: &BuildConfig,
    extra_env: &[(&str, &str)],
) -> Result<(), BuildError>
```

This allows tests to shadow PATH without affecting the system environment.

## Design Rationale

### Why Subprocess Instead of Library?

Linking against `wasm-bindgen-cli-support` introduces significant version coupling:

- The CLI version must match the library version exactly
- Updating one requires updating the other
- Breaking changes are common between versions

Shelling out to `wasm-pack`:

- Decouples tool from wasm-bindgen versions
- Simpler dependency tree
- Users can update wasm-pack independently

### Why Not Use trunk?

trunk is an excellent tool, but:

- Declining maintenance activity
- Designed as a standalone dev server, not embeddable
- Doesn't integrate with Actix-web routing

flux-wasm-builder embeds the build logic directly into generated projects, giving full control over the server and routing.
