# flux-wasm-builder

A single-command dev tool for fullstack Rust web applications using **Actix-web** and **Yew**. Scaffolds a three-crate workspace, then stays alive as the persistent driver of your entire development loop.

## How it works

Most fullstack Rust setups require you to manage two separate processes: one for the frontend WASM build and one for the backend server. `flux-wasm-builder` replaces both with a single command that owns the whole loop:

```
flux-wasm-builder dev
```

This starts a dev server on port 8080 that:

- Builds the frontend with `wasm-pack` on startup
- Spawns your Actix-web backend and waits for it to be ready
- Proxies `/api/*` requests to the backend
- Watches `frontend/src/` and rebuilds WASM on changes
- Watches `backend/src/` and restarts the backend on changes
- Watches `frontend/styles/` and recompiles SCSS on changes
- Watches `frontend/public/` and triggers a browser reload on changes
- Signals the browser to reload after successful builds

The generated project contains only user code — API handlers, Yew components, shared types, and styles. The tool handles everything else.

## Prerequisites

- **Rust** (edition 2024)
- **wasm-pack** >= 0.13.0 — [install guide](https://rustwasm.github.io/wasm-pack/)
- **wasm32-unknown-unknown** target: `rustup target add wasm32-unknown-unknown`

The `init` command validates these before scaffolding.

## Installation

```bash
git clone https://github.com/your-username/flux-wasm-builder.git
cd flux-wasm-builder
cargo install --path .
```

Reinstall after making changes to the tool.

## Usage

### Create a new project

```bash
flux-wasm-builder init my-app
cd my-app
```

This creates a three-crate Cargo workspace:

```
my-app/
├── .cargo/config.toml      # cargo alias: backend = "run -p my-app-backend"
├── .gitignore
├── Cargo.toml              # workspace root
├── flux.toml               # flux-wasm-builder configuration
├── backend/                # Actix-web API server
│   ├── Cargo.toml
│   ├── build.rs
│   ├── configuration/      # YAML-based config (base, local, production)
│   ├── src/
│   │   ├── bin/main.rs
│   │   ├── lib.rs
│   │   ├── api/
│   │   ├── configuration.rs
│   │   ├── error.rs
│   │   ├── response.rs
│   │   ├── startup.rs
│   │   ├── static_assets.rs
│   │   └── telemetry.rs
│   └── tests/
│       └── api/            # integration test scaffolding
├── frontend/               # Yew WASM application
│   ├── Cargo.toml
│   ├── index.html
│   ├── public/             # static assets (served as-is)
│   ├── src/
│   └── styles/
│       └── screen.scss     # Josh Comeau CSS reset included
└── shared/                 # Serde-compatible API types
    ├── Cargo.toml
    └── src/
```

### Start the dev server

```bash
flux-wasm-builder dev
```

Open `http://localhost:8080`. Edits to frontend, backend, styles, or public assets are picked up automatically.

### Build for release

```bash
flux-wasm-builder release
```

This runs `wasm-pack build --release` on the frontend, then `cargo build --release --features embed-assets` on the backend. The result is a single self-contained binary with all frontend assets embedded — no external files required.

## Configuration

`flux.toml` lives at the workspace root. All `[dev]` fields are optional and fall back to the defaults shown:

```toml
[project]
name = "my-app"

[dev]
public_port = 8080          # browser-facing port
backend_port = 3001         # internal backend port
watch_debounce_ms = 300     # file change debounce interval
```

The backend reads its port from the `FLUX_BACKEND_PORT` environment variable, which `flux-wasm-builder dev` sets automatically from `config.dev.backend_port`.

## Architecture

```
Browser :8080
    │
    ├── /api/*              → proxy → Backend :3001
    ├── /pkg/*              → wasm-pack build output (filesystem)
    ├── /styles/screen.css  → compiled from frontend/styles/screen.scss
    ├── /ws/reload          → WebSocket live reload
    └── /*                  → index.html (SPA fallback)
```

The tool runs four file watchers simultaneously:

| Watcher | Path | Filters | On change |
|---------|------|---------|-----------|
| Frontend | `frontend/src/` | `.rs` | `wasm-pack build`, then browser reload |
| Backend | `backend/src/` | `.rs` | Kill backend, respawn, health check, then browser reload |
| Styles | `frontend/styles/` | `.scss` | Recompile SCSS in memory, browser reload |
| Public assets | `frontend/public/` | any file | Browser reload |

## Generated backend

The scaffolded backend is an opinionated Actix-web starter. Out of the box `backend/src/` contains:

- **`configuration.rs`** — YAML-based configuration loading via the `config` crate. Supports environment-specific overrides (`configuration/base.yaml`, `local.yaml`, `production.yaml`). The `FLUX_BACKEND_PORT` env var overrides the configured port at runtime.
- **`startup.rs`** — `Application` struct that wires up the Actix-web server, routes, and middleware.
- **`error.rs`** — `ApiError` enum implementing Actix-web's `ResponseError` trait, mapping variants (`BadRequest`, `NotFound`, `Internal`) to HTTP status codes.
- **`response.rs`** — `ApiResponse<T>` generic wrapper implementing the `Responder` trait, with `success()` and `error()` constructors for consistent JSON responses.
- **`telemetry.rs`** — Tracing subscriber setup with environment-based log filtering via `tracing` and `tracing-subscriber`.
- **`static_assets.rs`** — Serves embedded frontend assets in release builds. Gated on the `embed-assets` feature flag — not compiled during development.
- **`api/`** — Route configuration with starter endpoints: `/api/health_check`, `/api/hello`, and `/api/status`.

Integration test scaffolding lives in `tests/api/` with a `TestApp` helper that spawns the server on a random port, plus a sample health-check test to build on.

## The shared crate

`shared/` contains the API boundary types used by both backend and frontend. Both crates depend on it, so serialization mismatches are compile errors rather than runtime surprises:

```rust
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

## Feature flags (generated backend)

| Flag | Effect |
|------|--------|
| `embed-assets` | Embeds `frontend/pkg/`, `frontend/index.html`, `frontend/public/`, and compiled CSS into the binary at compile time |

Used automatically by `flux-wasm-builder release`.

## Running tests

```bash
cargo test
```

Tests that invoke `wasm-pack` are gated behind an environment variable:

```bash
RUN_WASM_TESTS=1 cargo test
```

## Non-goals

- Multi-framework support (Yew only)
- CSS-in-Rust or Tailwind integration
- npm or Node.js in the build path
- Multi-target or SSR builds

## License

MIT — see [LICENSE.txt](LICENSE.txt)
