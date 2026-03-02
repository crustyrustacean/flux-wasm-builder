# Specification

This document provides a comprehensive reference for the flux-wasm-builder design specification.

## Problem Statement

Building a fullstack web application with Actix-web on the backend and Yew on the frontend currently requires two parallel, independent build pipelines that have no awareness of each other. The conventional approach involves:

- Running `trunk serve` in one terminal to watch and compile the Yew frontend
- Running `cargo run` in a second terminal to start the Actix-web backend
- Configuring a reverse proxy to forward API requests between ports
- Managing a Cargo workspace with multiple crates

This arrangement compounds complexity as a project grows.

## Goals

- **Single entry point:** `cargo run` starts everything in development mode
- **Single binary deployment:** `cargo build --release --features embed-assets` produces a self-contained binary
- **Inline compiler errors:** Rust compilation errors appear in the same terminal
- **Live reload:** Saved changes trigger incremental rebuild and browser refresh
- **Shared types crate:** Common crate for API-boundary types
- **No JavaScript build tooling required:** Only `wasm-pack` and Rust toolchain needed
- **Self-contained build subsystem:** Build logic lives inside generated backend crate
- **Project scaffolding:** `init` command creates fully wired, runnable workspace

## Non-Goals

- **General-purpose frontend framework support:** Yew only
- **Multi-target builds:** Single frontend crate, single WASM binary
- **Public release or crates.io publication:** Personal productivity tool
- **CSS/SASS preprocessing:** Static CSS served as-is
- **npm ecosystem integration:** No Node.js required

## Architecture

### Tool Structure

```
flux-wasm-builder/
├── Cargo.toml
├── src/
│   ├── bin/main.rs           # CLI entry point
│   ├── lib.rs                # public API
│   ├── env_check.rs          # prerequisite validation
│   ├── init/
│   │   ├── mod.rs            # scaffold logic
│   │   └── templates.rs      # generated file contents
│   └── build/
│       ├── mod.rs
│       ├── build.rs          # wasm-pack invocation
│       ├── build_coordinator.rs
│       ├── watcher.rs
│       ├── reload.rs
│       └── static_assets.rs
└── tests/
```

### Generated Project Structure

```
my-app/
├── .cargo/config.toml
├── .gitignore
├── Cargo.toml
├── backend/
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── build_subsystem/
│       └── api/
├── frontend/
│   ├── Cargo.toml
│   ├── index.html
│   └── src/
└── shared/
    ├── Cargo.toml
    └── src/
```

## Runtime Modes

| Mode | Behavior |
|------|----------|
| `dev` (default) | wasm-pack build on startup, file watcher, WebSocket reload |
| `release` (`embed-assets`) | Assets embedded at compile time, no dev tooling |

## Request Routing

1. **API routes** (`/api/...`) — Application handlers (first)
2. **Static assets** (`/pkg/{filename}`) — WASM, JS, artifacts
3. **SPA fallback** — Returns `index.html` for client-side routing

## Key Dependencies

### Tool

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `tokio` | Async runtime |
| `thiserror` | Error types |
| `tracing` | Logging |

### Generated Backend

| Crate | Purpose |
|-------|---------|
| `actix-web` | HTTP server |
| `actix-ws` | WebSocket |
| `notify-debouncer-mini` | File watching |
| `include_dir` | Asset embedding (optional) |

### Generated Frontend

| Crate | Purpose |
|-------|---------|
| `yew` | Frontend framework (0.21) |
| `wasm-bindgen` | Rust↔JS FFI |
| `gloo-net` | HTTP client |

## Implementation Stages

| Stage | Description |
|-------|-------------|
| 0 | Project Scaffolding (`init`) |
| 1 | Build Pipeline (wasm-pack) |
| 2 | File Watching |
| 3 | Live Reload |
| 4 | Static Asset Serving |
| 5 | Shared Types |
| 6 | Production Embedding |

## Security

- Path traversal protection via `file_name()` extraction
- MIME type safety via `mime_guess`
- WASM served with `application/wasm`

## Configuration

```rust
BuildConfig {
    frontend_crate_path: PathBuf,
    pkg_output_path: PathBuf,
    index_html_path: PathBuf,
    watch_debounce_ms: u64,      // default: 300
    reload_ws_path: String,      // default: "/ws/reload"
    port: u16,                   // default: 8080
    build_timeout_secs: u64,     // default: 300
}
```

## Error Handling

All errors are typed via `thiserror`:

- `BuildError` — wasm-pack failures
- `InitError` — Scaffolding failures
- `EnvCheckError` — Prerequisite failures

## Testing

- Unit tests in source files
- Integration tests in `tests/`
- TDD approach: write tests first
- `RUN_WASM_TESTS=1` for wasm-pack tests
