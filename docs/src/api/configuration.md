# Configuration Types

Configuration is handled through the `BuildConfig` struct.

## BuildConfig

```rust
#[derive(Clone, Debug)]
pub struct BuildConfig {
    /// Path to the frontend crate directory (e.g., "../frontend")
    pub frontend_crate_path: PathBuf,
    
    /// Path to the pkg output directory (default: frontend_crate_path + "/pkg")
    pub pkg_output_path: PathBuf,
    
    /// Path to index.html (default: frontend_crate_path + "/index.html")
    pub index_html_path: PathBuf,
    
    /// Watch debounce interval in milliseconds (default: 300)
    pub watch_debounce_ms: u64,
    
    /// WebSocket path for reload signaling (default: "/ws/reload")
    pub reload_ws_path: String,
    
    /// Server port (default: 8080)
    pub port: u16,
    
    /// Build timeout in seconds (default: 300)
    pub build_timeout_secs: u64,
}
```

## Creating Configuration

### With Defaults

```rust
let config = BuildConfig::new("../frontend");
```

This sets:
- `pkg_output_path` = `../frontend/pkg`
- `index_html_path` = `../frontend/index.html`
- `watch_debounce_ms` = `300`
- `reload_ws_path` = `"/ws/reload"`
- `port` = `8080`
- `build_timeout_secs` = `300`

### With Custom Values

```rust
let config = BuildConfig {
    port: 3000,
    build_timeout_secs: 600,
    watch_debounce_ms: 500,
    ..BuildConfig::new("../frontend")
};
```

### With Environment Variables

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

## Field Details

### frontend_crate_path

**Type:** `PathBuf`
**Required:** Yes

Path to the frontend crate directory. This is passed to `wasm-pack build`.

```rust
let config = BuildConfig::new("../frontend");
// or
let config = BuildConfig::new("/absolute/path/to/frontend");
```

### pkg_output_path

**Type:** `PathBuf`
**Default:** `{frontend_crate_path}/pkg`

Where wasm-pack outputs build artifacts. Used by `serve_pkg_file` to locate assets.

```rust
let config = BuildConfig {
    pkg_output_path: PathBuf::from("../frontend/pkg"),
    ..BuildConfig::new("../frontend")
};
```

### index_html_path

**Type:** `PathBuf`
**Default:** `{frontend_crate_path}/index.html`

Path to the index.html file. Used by `spa_fallback` to serve the SPA entry point.

```rust
let config = BuildConfig {
    index_html_path: PathBuf::from("../frontend/index.html"),
    ..BuildConfig::new("../frontend")
};
```

### watch_debounce_ms

**Type:** `u64`
**Default:** `300`

Debounce interval for file watching. After a file change is detected, the watcher waits this many milliseconds before signaling a rebuild.

**Trade-offs:**
- Lower values = faster feedback, but may trigger duplicate builds
- Higher values = fewer builds, but slower feedback

```rust
let config = BuildConfig {
    watch_debounce_ms: 500,  // More conservative
    ..BuildConfig::new("../frontend")
};
```

### reload_ws_path

**Type:** `String`
**Default:** `"/ws/reload"`

WebSocket endpoint path for live reload. Must match the path in the injected script.

```rust
let config = BuildConfig {
    reload_ws_path: "/livereload".to_string(),
    ..BuildConfig::new("../frontend")
};
```

**Note:** If you change this, also update `RELOAD_SCRIPT` in `reload.rs`.

### port

**Type:** `u16`
**Default:** `8080`

TCP port for the HTTP server. If the port is already in use, the server will fail to start with an error message.

```rust
let config = BuildConfig {
    port: 3000,
    ..BuildConfig::new("../frontend")
};
```

### build_timeout_secs

**Type:** `u64`
**Default:** `300`

Maximum time to wait for a wasm-pack build. If exceeded, the build process is killed and a timeout error is returned.

```rust
let config = BuildConfig {
    build_timeout_secs: 600,  // 10 minutes for large projects
    ..BuildConfig::new("../frontend")
};
```

## DevMode

```rust
#[derive(Clone, Copy, Debug)]
pub struct DevMode(pub bool);
```

A simple flag indicating whether the server is running in development mode.

- `DevMode(true)`: Inject reload script, enable WebSocket endpoint
- `DevMode(false)`: Serve index.html as-is

```rust
let dev_mode = DevMode(!cfg!(feature = "embed-assets"));

HttpServer::new(move || {
    App::new()
        .app_data(web::Data::new(dev_mode))
        .default_service(web::get().to(spa_fallback))
})
```

## Configuration Best Practices

### Use Environment Variables for Deployment

```rust
let config = BuildConfig {
    port: env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080),
    ..BuildConfig::new("../frontend")
};
```

This allows the same binary to run on different ports without recompilation.

### Validate Configuration Early

```rust
fn validate_config(config: &BuildConfig) -> Result<(), String> {
    if !config.frontend_crate_path.exists() {
        return Err(format!(
            "Frontend crate not found: {}",
            config.frontend_crate_path.display()
        ));
    }
    if config.port == 0 {
        return Err("Port cannot be 0".to_string());
    }
    Ok(())
}
```

### Share Configuration Across Handlers

```rust
HttpServer::new(move || {
    App::new()
        .app_data(web::Data::new(config.clone()))
        .app_data(web::Data::new(dev_mode))
        .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        .default_service(web::get().to(spa_fallback))
})
```

Using `web::Data` ensures all handlers access the same configuration.
