# flux-wasm-builder

A single-command build tool for fullstack Rust web applications using **Actix-web** (backend) and **Yew** (frontend). Eliminate dual-pipeline complexity — one `cargo run` builds, serves, and watches your entire application.

## Overview

Building fullstack Rust applications typically requires managing two separate build pipelines: `trunk serve` for the Yew frontend and `cargo run` for the Actix-web backend. **flux-wasm-builder** consolidates this into a unified workflow:

- **Single entry point**: `cargo run` starts everything in development mode
- **Single binary deployment**: `cargo build --release --features embed-assets` produces a self-contained binary
- **Live reload**: Frontend changes trigger automatic browser refresh
- **Shared types**: A dedicated crate for API-boundary types with compile-time guarantees

## Features

### Project Scaffolding

```bash
flux-wasm-builder init my-app
cd my-app
cargo backend
```

Creates a fully-wired three-crate Cargo workspace:

```
my-app/
├── .cargo/config.toml      # cargo alias: backend = "run -p my-app-backend"
├── .gitignore              # excludes target/ and frontend/pkg/
├── Cargo.toml              # workspace root
├── backend/                # Actix-web server with embedded build subsystem
│   ├── Cargo.toml
│   ├── build.rs            # compile-time guard for embed-assets feature
│   └── src/
│       ├── main.rs
│       ├── build_subsystem/  # copied from tool templates
│       └── api/
├── frontend/               # Yew WASM application
│   ├── Cargo.toml
│   ├── index.html
│   └── src/
└── shared/                 # API types (serde-compatible)
    ├── Cargo.toml
    └── src/
```

### Development Mode

In development mode (default):

1. On startup, invokes `wasm-pack build --target web --dev`
2. Starts Actix-web server on port 8080
3. Watches `frontend/src/**/*.rs` for changes
4. On change: rebuilds frontend and signals browser to reload via WebSocket

### Production Mode

With the `embed-assets` feature:

```bash
wasm-pack build frontend/ --target web --release
cargo build --release --features embed-assets
```

Produces a single binary with embedded WASM and JavaScript assets — no external file dependencies.

### Live Reload

The development server injects a WebSocket client script into `index.html` at serve time:

- Automatic reconnection on server restart
- Page reload after successful frontend rebuild
- No manual refresh required

### Shared Types Crate

The `shared` crate enforces API contracts at the type level:

```rust
// shared/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
}
```

Both backend and frontend depend on this crate, ensuring serialization mismatches are compile errors.

## Prerequisites

- **Rust** (edition 2024 compatible)
- **wasm-pack** >= 0.13.0 — [install guide](https://rustwasm.github.io/wasm-pack/)
- **wasm32-unknown-unknown** target: `rustup target add wasm32-unknown-unknown`

The `init` command validates these prerequisites and provides actionable error messages if missing.

## Installation

From source:

```bash
git clone https://github.com/your-username/flux-wasm-builder.git
cd flux-wasm-builder
cargo install --path .
```

## Usage

### Create a New Project

```bash
flux-wasm-builder init my-app
```

Options:
- `--path, -p <DIR>` — Parent directory (default: current directory)

### Development Server

```bash
cd my-app
cargo backend
```

This:
1. Builds the frontend with `wasm-pack`
2. Starts the Actix-web server on `http://localhost:8080`
3. Watches for frontend source changes
4. Triggers rebuilds and browser reloads automatically

### Production Build

```bash
# Build frontend for release
wasm-pack build frontend/ --target web --release

# Build backend with embedded assets
cargo build --release --features embed-assets
```

The resulting binary is self-contained and deployable without external files.

## Architecture

### Request Routing

The Actix-web server handles requests in this order:

1. **API routes** (`/api/...`) — Application handlers, registered first
2. **Static assets** (`/pkg/{filename}`) — WASM, JS, and other build artifacts
3. **SPA fallback** — All unmatched routes return `index.html` for client-side routing

### Build Subsystem

The build subsystem is embedded directly into the generated backend crate:

| Module | Purpose |
|--------|---------|
| [`build.rs`](src/build/build.rs) | wasm-pack subprocess invocation with timeout |
| [`build_coordinator.rs`](src/build/build_coordinator.rs) | Serialized rebuild loop with coalescing |
| [`watcher.rs`](src/build/watcher.rs) | Filesystem watching with debouncing |
| [`reload.rs`](src/build/reload.rs) | WebSocket broadcast for live reload |
| [`static_assets.rs`](src/build/static_assets.rs) | Asset serving with path traversal protection |

### Feature Flags

| Flag | Description |
|------|-------------|
| `embed-assets` | Embeds `frontend/pkg/` and `index.html` into binary at compile time |

When `embed-assets` is active:
- File watcher is compiled out
- WebSocket endpoint is compiled out
- Assets served from embedded bytes, not filesystem

## Configuration

Build configuration is defined in `backend/src/main.rs`:

```rust
BuildConfig {
    frontend_crate_path: PathBuf::from("../frontend"),
    pkg_output_path: PathBuf::from("../frontend/pkg"),
    index_html_path: PathBuf::from("../frontend/index.html"),
    watch_debounce_ms: 300,        // file change debounce interval
    reload_ws_path: "/ws/reload",  // WebSocket endpoint
    port: 8080,                    // server port
    build_timeout_secs: 300,       // wasm-pack timeout
}
```

## API Reference

### Library Exports

```rust
// Build subsystem
pub use flux_wasm_builder::{
    BuildConfig, BuildError, 
    run_wasm_pack, run_wasm_pack_with_env,
    run_command_with_timeout,
    start_watcher, FileWatcher,
    serve_pkg_file, spa_fallback,
};

// Dev-mode only (not available with embed-assets feature)
pub use flux_wasm_builder::{
    inject_reload_script, ws_reload_handler, RELOAD_SCRIPT,
};

// Initialization
pub use flux_wasm_builder::{
    scaffold, InitError,
};

// Environment checking
pub use flux_wasm_builder::{
    EnvCheckError,
};
```

### Key Types

#### `BuildConfig`

Configuration for the build subsystem. Use [`BuildConfig::new()`](src/build/build.rs:57) for defaults.

#### `BuildError`

```rust
pub enum BuildError {
    WasmPackFailed { exit_code: Option<i32> },
    Timeout { secs: u64 },
    SpawnFailed { source: std::io::Error },
    WaitFailed { source: std::io::Error },
    KillFailed { source: std::io::Error },
}
```

#### `InitError`

```rust
pub enum InitError {
    AlreadyExists(PathBuf),
    EnvCheck(EnvCheckError),
    CreateDir { path: PathBuf, source: std::io::Error },
    WriteFile { path: PathBuf, source: std::io::Error },
}
```

## Testing

Run all tests:

```bash
cargo test
```

Run tests requiring wasm-pack:

```bash
RUN_WASM_TESTS=1 cargo test
```

### Test Categories

| File | Coverage |
|------|----------|
| [`tests/init.rs`](tests/init.rs) | Project scaffolding |
| [`tests/build.rs`](tests/build.rs) | Build subsystem |
| [`tests/serve.rs`](tests/serve.rs) | Static asset serving |
| [`tests/watcher.rs`](tests/watcher.rs) | File watching and coordination |
| [`tests/reload.rs`](tests/reload.rs) | Live reload functionality |
| [`tests/api.rs`](tests/api.rs) | Shared types and API handlers |

## Security

### Path Traversal Protection

The [`serve_pkg_file`](src/build/static_assets.rs:46) handler uses `file_name()` to extract only the final path component, preventing attacks like `/pkg/../../etc/passwd`.

### MIME Type Safety

All served files use MIME types detected via `mime_guess`. WASM files are served with `application/wasm` for proper browser streaming instantiation.

## Dependencies

### Tool Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `tokio` | Async runtime |
| `thiserror` | Error type derivation |
| `tracing` | Structured logging |
| `actix-web` | HTTP server |
| `actix-ws` | WebSocket handling |
| `notify-debouncer-mini` | Filesystem watching |

### Generated Backend Dependencies

| Crate | Purpose |
|-------|---------|
| `actix-web` | HTTP server, routing |
| `actix-ws` | WebSocket for live reload |
| `tokio` | Async runtime |
| `notify-debouncer-mini` | File watching |
| `include_dir` | Asset embedding (optional) |
| `serde_json` | JSON serialization |
| `mime_guess` | MIME type detection |
| `tracing` | Logging |

### Generated Frontend Dependencies

| Crate | Purpose |
|-------|---------|
| `yew` | Frontend framework (0.21, CSR mode) |
| `wasm-bindgen` | Rust↔JS FFI |
| `gloo-net` | Async HTTP client |
| `web-sys` | DOM API bindings |
| `wasm_logger` | Browser console logging |
| `console_error_panic_hook` | Panic reporting |

## Non-Goals

- **Multi-framework support**: Yew only, no React/Svelte/etc.
- **Multi-target builds**: Single frontend crate, single WASM binary
- **CSS preprocessing**: Static CSS served as-is
- **npm integration**: No Node.js required in the build path

## Contributing

1. Fork the repository
2. Create a feature branch
3. Write tests first (TDD)
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

### Development Setup

```bash
git clone https://github.com/your-username/flux-wasm-builder.git
cd flux-wasm-builder
cargo build
cargo test
```

## License

This project is licensed under the MIT License - see the [LICENSE.txt](LICENSE.txt) file for details.

## Acknowledgments

Built for the Rust fullstack ecosystem:
- [Actix-web](https://actix.rs/) — Powerful, pragmatic web framework
- [Yew](https://yew.rs/) — Rust/WASM frontend framework
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) — Rust↔WASM build tool
