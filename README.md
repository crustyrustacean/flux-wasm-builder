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
│   └── src/
│       ├── main.rs
│       └── api/
├── frontend/               # Yew WASM application
│   ├── Cargo.toml
│   ├── index.html
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

Open `http://localhost:8080`. Edits to frontend, backend, or styles are picked up automatically.

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
    ├── /api/*          → proxy → Backend :3001
    ├── /pkg/*          → wasm-pack build output (filesystem)
    ├── /styles/screen.css → compiled from frontend/styles/screen.scss
    ├── /ws/reload      → WebSocket live reload
    └── /*              → index.html (SPA fallback)
```

The tool runs three file watchers simultaneously:

| Watcher | Path | On change |
|---------|------|-----------|
| Frontend | `frontend/src/` | `wasm-pack build`, then browser reload |
| Backend | `backend/src/` | Kill backend, respawn, health check |
| Styles | `frontend/styles/` | Recompile SCSS in memory, browser reload |

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
| `embed-assets` | Embeds `frontend/pkg/` and `frontend/index.html` into the binary at compile time |

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
