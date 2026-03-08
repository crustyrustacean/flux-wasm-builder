# Docker

A `Dockerfile` is included in every project scaffolded by `wasm-drydock init`. It uses a multi-stage build to produce a minimal production image.

## Build stages

**`chef`** — base image with all build tools installed:
- Rust toolchain
- wasm-pack
- wasm32-unknown-unknown target
- cargo-chef for dependency caching

**`planner`** — analyses the dependency graph and produces `recipe.json`

**`builder`** — cooks dependencies from `recipe.json`, then builds the project:
1. `wasm-pack build --release` on the frontend
2. `grass` compiles SCSS to CSS
3. `cargo build --release --features embed-assets` on the backend

**Final stage** — copies only the release binary into a minimal `debian:bookworm-slim` image.

## Building

```bash
docker build -t my-app .
```

## Running

```bash
docker run -p 3001:3001 \
  -e APP_ENVIRONMENT=production \
  -e APP_APPLICATION__HOST=0.0.0.0 \
  my-app
```
