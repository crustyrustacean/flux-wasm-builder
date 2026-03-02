# Introduction

**flux-wasm-builder** is a single-command build tool for fullstack Rust web applications using **Actix-web** (backend) and **Yew** (frontend).

## The Problem

Building a fullstack web application with Actix-web on the backend and Yew on the frontend typically requires two parallel, independent build pipelines:

- Running `trunk serve` in one terminal to watch and compile the Yew frontend
- Running `cargo run` in a second terminal to start the Actix-web backend
- Configuring a reverse proxy to forward API requests between ports
- Managing a Cargo workspace with multiple crates

This arrangement compounds complexity as a project grows:

- Two watch processes compete for the same source tree
- Proxy configuration is a third artifact to maintain
- Shared data types must be duplicated or extracted into a third crate
- Deployment involves coordinating multiple build artifacts

## The Solution

**flux-wasm-builder** eliminates this dual-pipeline complexity:

- **Single entry point**: `cargo run` starts everything in development mode
- **Single binary deployment**: `cargo build --release --features embed-assets` produces a self-contained binary
- **Live reload**: Frontend changes trigger automatic browser refresh
- **Shared types**: A dedicated crate for API-boundary types with compile-time guarantees

## Key Features

### Project Scaffolding

```bash
flux-wasm-builder init my-app
cd my-app
cargo backend
```

Creates a fully-wired three-crate Cargo workspace with backend, frontend, and shared crates.

### Development Mode

- On startup, invokes `wasm-pack build --target web --dev`
- Starts Actix-web server on port 8080
- Watches `frontend/src/**/*.rs` for changes
- On change: rebuilds frontend and signals browser to reload via WebSocket

### Production Mode

```bash
wasm-pack build frontend/ --target web --release
cargo build --release --features embed-assets
```

Produces a single binary with embedded WASM and JavaScript assets.

### Live Reload

The development server injects a WebSocket client script into `index.html` at serve time, enabling automatic page refresh after successful frontend rebuilds.

## Non-Goals

- **Multi-framework support**: Yew only, no React/Svelte/etc.
- **Multi-target builds**: Single frontend crate, single WASM binary
- **CSS preprocessing**: Static CSS served as-is
- **npm integration**: No Node.js required in the build path

## License

This project is licensed under the MIT License - see the [LICENSE.txt](../LICENSE.txt) file for details.
