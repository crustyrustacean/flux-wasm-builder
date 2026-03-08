# wasm-drydock

A single-command dev tool for fullstack Rust web applications using **Actix-web** and **Yew**. Scaffolds a three-crate workspace, then stays alive as the persistent driver of your entire development loop.

## How it works

Most fullstack Rust setups require you to manage two separate processes: one for the frontend WASM build and one for the backend server. `wasm-drydock` replaces both with a single command that owns the whole loop:

```
wasm-drydock dev
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
git clone https://github.com/your-username/wasm-drydock.git
cd wasm-drydock
cargo install --path .
```

Reinstall after making changes to the tool.

## Usage

### Create a new project

```bash
wasm-drydock init my-app
cd my-app
```

This creates a three-crate Cargo workspace:

```
my-app/
├── .cargo/config.toml      # cargo alias: backend = "run -p my-app-backend"
├── .gitignore
├── Cargo.toml              # workspace root
├── drydock.toml            # wasm-drydock configuration
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
wasm-drydock dev
```

Open `http://localhost:8080`. Edits to frontend, backend, styles, or public assets are picked up automatically.

To open the browser automatically when the server starts:

```bash
wasm-drydock dev --open
# or
wasm-drydock dev -o
```

### Build for release

```bash
wasm-drydock release
```

This runs `wasm-pack build --release` on the frontend, then `cargo build --release --features embed-assets` on the backend. The result is a single self-contained binary — all frontend assets **and** `configuration/base.yaml` are compiled in, so the binary runs with zero filesystem dependencies.

Deploy the binary alone. To customise settings without a recompile, place an environment-specific YAML file (e.g. `configuration/production.yaml`) next to the binary or anywhere in its walk-up path. It is loaded on top of the embedded base config when present. `APP_*` environment variables are always applied last and override everything.

## Configuration

`drydock.toml` lives at the workspace root. All `[dev]` fields are optional and fall back to the defaults shown:

```toml
[project]
name = "my-app"

[dev]
public_port = 8080          # browser-facing port
backend_port = 3001         # internal backend port
watch_debounce_ms = 300     # file change debounce interval
```

The backend reads its port from the `DRYDOCK_BACKEND_PORT` environment variable, which `wasm-drydock dev` sets automatically from `config.dev.backend_port`.

### Custom Watch Paths

You can add custom watch paths for additional file monitoring using `[[watch]]` entries:

```toml
# Watch a content folder for markdown files
[[watch]]
path = "frontend/content"
extensions = ["md", "mdx"]
action = "reload"

# Watch an assets folder (all file types)
[[watch]]
path = "frontend/assets"
extensions = []
action = "reload"

# Watch shared crate for type changes
[[watch]]
path = "shared/src"
extensions = ["rs"]
action = "rebuild"
```

**Available actions:**

| Action | Effect |
|--------|--------|
| `reload` | Trigger browser page reload |
| `rebuild` | Trigger WASM rebuild (then reload) |
| `restart` | Trigger backend restart |

**Notes:**
- Paths are relative to the project root
- Empty `extensions` array watches all files
- Non-existent paths are logged as warnings and skipped
- Core watchers (`frontend/src`, `backend/src`, `frontend/public`, `frontend/styles`) are always active

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

The tool runs four core file watchers simultaneously (always active):

| Watcher | Path | Filters | On change |
|---------|------|---------|-----------|
| Frontend | `frontend/src/` | `.rs` | `wasm-pack build`, then browser reload |
| Backend | `backend/src/` | `.rs` | Kill backend, respawn, health check, then browser reload |
| Styles | `frontend/styles/` | `.scss` | Recompile SCSS in memory, browser reload |
| Public assets | `frontend/public/` | any file | Browser reload |

Additional custom watchers can be configured via `[[watch]]` entries in `drydock.toml` (see [Custom Watch Paths](#custom-watch-paths)).

## Generated backend

The scaffolded backend is an opinionated Actix-web starter. Out of the box `backend/src/` contains:

- **`configuration.rs`** — YAML-based configuration loading via the `config` crate. In development builds, reads `configuration/base.yaml` and an environment-specific file from the filesystem. In release builds (`embed-assets` feature), `base.yaml` is compiled directly into the binary via `include_str!` — no config files required at runtime. An optional environment-specific file (e.g. `configuration/production.yaml`) placed next to the binary is still loaded when present. `APP_*` environment variables override everything. The `DRYDOCK_BACKEND_PORT` env var overrides the configured port at runtime.
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
| `embed-assets` | Embeds `frontend/pkg/`, `frontend/index.html`, `frontend/public/`, compiled CSS, **and `configuration/base.yaml`** into the binary at compile time. The resulting binary has zero runtime filesystem dependencies. |

Used automatically by `wasm-drydock release`.

## Running tests

```bash
cargo test
```

Tests that invoke `wasm-pack` are gated behind an environment variable:

```bash
RUN_WASM_TESTS=1 cargo test
```

## Someday goals

- Multi-framework support (Yew only at the moment, would be nice to add anything `trunk` supports)
- CSS-in-Rust or Tailwind integration
- Multi-target or SSR builds

## License

MIT — see [LICENSE.txt](LICENSE.txt)
