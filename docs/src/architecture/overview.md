# Architecture Overview

flux-wasm-builder consists of two main components:

1. **The tool itself** — A CLI application that scaffolds new projects
2. **The build subsystem** — Embedded into generated projects at init time

## Tool Structure

```
flux-wasm-builder/
├── Cargo.toml
├── src/
│   ├── bin/
│   │   └── main.rs           # CLI entry point (clap parsing)
│   ├── lib.rs                # Public API exports
│   ├── env_check.rs          # Prerequisite validation
│   ├── init/
│   │   ├── mod.rs            # Scaffolding logic
│   │   └── templates.rs      # Generated file contents
│   └── build/
│       ├── mod.rs            # Module exports
│       ├── build.rs          # wasm-pack invocation
│       ├── build_coordinator.rs  # Rebuild loop
│       ├── watcher.rs        # File watching
│       ├── reload.rs         # WebSocket live reload
│       └── static_assets.rs  # Asset serving
└── tests/
    ├── init.rs
    ├── build.rs
    ├── serve.rs
    ├── watcher.rs
    ├── reload.rs
    └── api.rs
```

## Generated Project Structure

```
my-app/
├── .cargo/config.toml        # cargo alias
├── .gitignore
├── Cargo.toml                # workspace root
├── backend/
│   ├── Cargo.toml
│   ├── build.rs              # embed-assets guard
│   └── src/
│       ├── main.rs
│       ├── build_subsystem/  # copied from templates
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

### Development Mode (Default)

```
cargo run          (resolved via .cargo/config.toml alias)
  └─► backend/src/main.rs  [#[actix_web::main]]
        ├─► env_check: wasm-pack >= 0.13.0, wasm32 target
        ├─► BuildSubsystem::start()
        │     ├─► wasm-pack build frontend/ --target web --dev
        │     ├─► notify_debouncer: watches frontend/src/**/*.rs
        │     └─► tokio::spawn: build_coordinator loop
        └─► actix_web::HttpServer::new()
              ├─► /api/...          (app handlers)
              ├─► /pkg/{filename}   (wasm + JS assets)
              ├─► /ws/reload        (WebSocket)
              └─► default_service   (SPA fallback)
```

### Production Mode (embed-assets feature)

```
cargo build --release --features embed-assets
  └─► build.rs validates frontend/pkg/ exists
  └─► include_dir embeds assets at compile time
  └─► Result: single self-contained binary
```

## Request Routing

The Actix-web server handles requests in priority order:

| Priority | Route | Handler | Description |
|----------|-------|---------|-------------|
| 1 | `/api/*` | Application | User-defined API routes |
| 2 | `/pkg/{filename}` | Static | WASM, JS, and build artifacts |
| 3 | `/ws/reload` | WebSocket | Live reload signaling (dev only) |
| 4 | `*` | SPA Fallback | Returns `index.html` |

This order ensures:

- API routes always take precedence
- Static assets are served correctly
- Client-side routing works for all other paths

## Key Design Decisions

### No External Config Files

Configuration is Rust code in `main.rs`. Benefits:

- Type safety
- IDE support
- No config file parsing
- Environment variable integration

### wasm-pack Subprocess

The tool shells out to `wasm-pack` rather than linking against `wasm-bindgen-cli-support`:

- Avoids version coupling friction
- Simpler dependency management
- Real-time output streaming

### Embedded Build Subsystem

The build subsystem is copied into generated projects:

- No runtime dependency on the tool
- Projects own their build logic
- Can be customized per-project

### Broadcast Channel for Reload

Uses `tokio::sync::broadcast` for reload signaling:

- Multiple browser tabs supported
- Non-blocking send (ignores no receivers)
- Clean shutdown handling

## Feature Flags

| Flag | Effect |
|------|--------|
| `embed-assets` | Embeds frontend assets into binary at compile time |

When `embed-assets` is active:

- File watcher is compiled out
- WebSocket endpoint is compiled out
- Assets served from embedded bytes

## Dependencies

### Tool Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `tokio` | Async runtime |
| `thiserror` | Error types |
| `tracing` | Logging |
| `actix-web` | HTTP server |
| `actix-ws` | WebSocket |
| `notify-debouncer-mini` | File watching |

### Generated Backend Dependencies

| Crate | Purpose |
|-------|---------|
| `actix-web` | HTTP server |
| `actix-ws` | WebSocket |
| `tokio` | Async runtime |
| `notify-debouncer-mini` | File watching |
| `include_dir` | Asset embedding (optional) |
| `serde_json` | JSON serialization |
| `mime_guess` | MIME types |
| `tracing` | Logging |

### Generated Frontend Dependencies

| Crate | Purpose |
|-------|---------|
| `yew` | Frontend framework |
| `wasm-bindgen` | Rust↔JS FFI |
| `gloo-net` | HTTP client |
| `web-sys` | DOM bindings |

## Next Steps

- [Build Subsystem](./build-subsystem.md) — wasm-pack invocation details
- [File Watching](./file-watching.md) — Change detection architecture
- [Live Reload](./live-reload.md) — WebSocket reload mechanism
- [Static Assets](./static-assets.md) — Asset serving and security
