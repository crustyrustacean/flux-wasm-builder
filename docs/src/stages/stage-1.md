# Stage 1: Build Pipeline

Stage 1 implements the build subsystem's core function: invoking `wasm-pack` as a subprocess.

## Overview

**Key Behavior:**
- On startup, invokes `wasm-pack build frontend/ --target web --dev`
- stdout and stderr are streamed directly to the parent process
- 300-second default timeout prevents indefinite hangs
- Build failure at startup exits the process with non-zero code
- Server is not started if initial build fails

## Architecture

```
cargo run (in generated project)
  └─► backend/src/main.rs  [#[actix_web::main]]
        ├─► BuildConfig::new("../frontend")
        ├─► run_wasm_pack(&config).await
        │     ├─► tokio::process::Command::new("wasm-pack")
        │     │     .args(["build", "--target", "web", "--dev"])
        │     │     .stdout(Stdio::inherit())
        │     │     .stderr(Stdio::inherit())
        │     ├─► tokio::time::timeout(Duration::from_secs(300), child.wait())
        │     └─► On timeout: child.kill().await
        └─► If build succeeded: HttpServer::new(...).bind(...)
              Else: exit(1)
```

## Implementation Components

### BuildConfig

```rust
pub struct BuildConfig {
    pub frontend_crate_path: PathBuf,
    pub pkg_output_path: PathBuf,
    pub index_html_path: PathBuf,
    pub watch_debounce_ms: u64,
    pub reload_ws_path: String,
    pub port: u16,
    pub build_timeout_secs: u64,
}
```

### BuildError

```rust
pub enum BuildError {
    WasmPackFailed { exit_code: Option<i32> },
    Timeout { secs: u64 },
    SpawnFailed { source: std::io::Error },
    WaitFailed { source: std::io::Error },
    KillFailed { source: std::io::Error },
}
```

### run_wasm_pack

```rust
pub async fn run_wasm_pack(config: &BuildConfig) -> Result<(), BuildError>
```

Spawns wasm-pack as a subprocess with:
- Inherited stdout/stderr for real-time output
- Configurable timeout
- Process killing on timeout

## Key Design Decisions

### Why Subprocess Instead of Library?

Linking against `wasm-bindgen-cli-support` introduces version coupling:
- CLI version must match library version exactly
- Breaking changes are common between versions

Shelling out to `wasm-pack`:
- Decouples tool from wasm-bindgen versions
- Simpler dependency tree
- Users can update wasm-pack independently

### Why Inherit Stdio?

Streaming output directly to the parent process:
- Rust compiler diagnostics appear inline
- No buffering delays
- Real-time feedback during builds

## Test Coverage

- wasm-pack builds scaffolded frontend (with `RUN_WASM_TESTS=1`)
- Scaffolded backend has build subsystem module
- Scaffolded backend main calls build subsystem
- Scaffolded backend exits on build failure
- Build timeout kills process
- Build error types are correct

## Usage in Generated Project

```rust
// backend/src/main.rs
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = BuildConfig::new("../frontend");
    
    // Initial build
    if let Err(e) = run_wasm_pack(&config).await {
        eprintln!("initial build failed: {}", e);
        std::process::exit(1);
    }
    
    // Start server
    HttpServer::new(|| { /* ... */ })
        .bind(("127.0.0.1", config.port))?
        .run()
        .await
}
```
