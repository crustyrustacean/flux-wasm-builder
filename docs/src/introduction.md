# wasm-drydock

wasm-drydock is a single-command dev tool for fullstack Rust web applications using **Actix-web** and **Yew**.

Most fullstack Rust setups require you to manage two separate processes: one for the frontend WASM build and one for the backend server. wasm-drydock replaces both with a single command that owns the entire development loop.

## What it does

```
wasm-drydock dev
```

This starts a dev server that:

- Builds the frontend with `wasm-pack` on startup
- Spawns your Actix-web backend and waits for it to be ready
- Proxies `/api/*` requests to the backend
- Watches source files and rebuilds on changes
- Recompiles SCSS on the fly
- Pushes build errors directly to the browser
- Signals the browser to reload after successful builds

When you are ready to ship, `wasm-drydock release` produces a single self-contained binary with all frontend assets compiled in — no separate static file server required.

## Prerequisites

- **Rust** (edition 2024)
- **wasm-pack** >= 0.13.0
- **wasm32-unknown-unknown** target: `rustup target add wasm32-unknown-unknown`
