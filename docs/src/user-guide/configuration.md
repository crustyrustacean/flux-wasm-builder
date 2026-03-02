# Configuration

flux-wasm-builder uses code-based configuration rather than external config files. All settings are defined in `backend/src/main.rs`.

## BuildConfig

The `BuildConfig` struct controls build and server behavior:

```rust
use build_subsystem::build::BuildConfig;

let config = BuildConfig {
    frontend_crate_path: PathBuf::from("../frontend"),
    pkg_output_path: PathBuf::from("../frontend/pkg"),
    index_html_path: PathBuf::from("../frontend/index.html"),
    watch_debounce_ms: 300,
    port: 8080,
    build_timeout_secs: 300,
};
```

### Configuration Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `frontend_crate_path` | `PathBuf` | Required | Path to frontend crate directory |
| `pkg_output_path` | `PathBuf` | `{frontend_crate_path}/pkg` | wasm-pack output directory |
| `index_html_path` | `PathBuf` | `{frontend_crate_path}/index.html` | Path to index.html |
| `watch_debounce_ms` | `u64` | `300` | File change debounce interval |
| `port` | `u16` | `8080` | Server port |
| `build_timeout_secs` | `u64` | `300` | wasm-pack timeout in seconds |

### Using Defaults

Create a config with defaults for all optional fields:

```rust
let config = BuildConfig::new("../frontend");
```

This sets all other fields to their default values.

## Environment-Based Configuration

For production deployments, consider using environment variables:

```rust
use std::env;

let config = BuildConfig {
    port: env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080),
    build_timeout_secs: env::var("BUILD_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(300),
    ..BuildConfig::new("../frontend")
};
```

## Port Configuration

### Default Port

The server binds to port 8080 by default.

### Custom Port

```rust
let config = BuildConfig {
    port: 3000,
    ..BuildConfig::new("../frontend")
};
```

### Port in Use Error

If the port is already bound:

```
error: port 8080 is already in use. Change the port in BuildConfig.
```

Solutions:

1. Change the port in configuration
2. Stop the conflicting process
3. Use environment variable for dynamic port assignment

## Watch Configuration

### Debounce Interval

The debounce interval controls how rapidly file changes trigger builds:

```rust
let config = BuildConfig {
    watch_debounce_ms: 500,  // Wait 500ms after last change
    ..BuildConfig::new("../frontend")
};
```

**Trade-offs:**

- **Lower values** (100-200ms): Faster feedback, but may trigger multiple builds for multi-file saves
- **Higher values** (500-1000ms): Fewer spurious builds, but slower feedback loop

The default 300ms works well for most editors' atomic save behavior.

### Watched Paths

By default, the watcher monitors `frontend/src/` recursively. Only `.rs` files trigger rebuilds.

To customize watched paths, modify the watcher initialization in `main.rs`:

```rust
// Watch additional directories
let _watcher = start_watcher(
    Path::new("../frontend/src"),
    config.watch_debounce_ms,
    build_tx.clone(),
)?;

// Add another watcher for shared types
let _watcher2 = start_watcher(
    Path::new("../shared/src"),
    config.watch_debounce_ms,
    build_tx,
)?;
```

## Build Timeout

The build timeout prevents hung `wasm-pack` processes:

```rust
let config = BuildConfig {
    build_timeout_secs: 600,  // 10 minutes for large projects
    ..BuildConfig::new("../frontend")
};
```

If a build exceeds the timeout:

1. The `wasm-pack` process is killed
2. A `BuildError::Timeout` is returned
3. The server continues running with previous assets

## WebSocket Path

The WebSocket endpoint for live reload is fixed at `/ws/reload`. This endpoint is automatically configured in development mode and the reload script is injected into `index.html` automatically.

## Logging Configuration

Logging uses `tracing-subscriber` with environment-based filtering:

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

Set via `RUST_LOG` environment variable:

```bash
# Show all debug messages
RUST_LOG=debug cargo backend

# Show only errors
RUST_LOG=error cargo backend

# Show debug for specific module
RUST_LOG=build_subsystem::watcher=debug cargo backend
```

## Complete Example

```rust
use actix_web::{web, App, HttpServer};
use build_subsystem::build::{BuildConfig, run_wasm_pack};
use build_subsystem::static_assets::{serve_pkg_file, spa_fallback};
use build_subsystem::DevMode;
use std::path::PathBuf;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    // Configuration with environment overrides
    let config = BuildConfig {
        frontend_crate_path: PathBuf::from("../frontend"),
        port: env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080),
        build_timeout_secs: env::var("BUILD_TIMEOUT")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(300),
        watch_debounce_ms: 300,
        ..BuildConfig::new("../frontend")
    };

    // ... rest of main
}
```
