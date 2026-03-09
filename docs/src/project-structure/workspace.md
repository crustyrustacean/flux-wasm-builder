# Workspace

A wasm-drydock project is a Cargo workspace containing three member crates: `backend`, `frontend`, and `shared`.

## `Cargo.toml`

The workspace manifest at the project root lists all three members and pins shared dependency versions in `[workspace.dependencies]`:

```toml
[workspace]
members = ["backend", "frontend", "shared"]
resolver = "2"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
```

## `.cargo/config.toml`

The workspace-level Cargo configuration sets the default build target for the `frontend` crate:

```toml
[build]
target = "wasm32-unknown-unknown"
```

> **Note:** This applies only when building inside the `frontend` crate directory. wasm-drydock handles target selection automatically when invoking `wasm-pack`.

## `drydock.toml`

The wasm-drydock configuration file at the workspace root. See [Configuration Reference](../configuration/reference.md) for all available fields.

## `.gitignore`

The generated `.gitignore` excludes:

- `target/` — Cargo build output
- `frontend/pkg/` — wasm-pack build output
- `frontend/styles/screen.css` — compiled SCSS (only written by `wasm-drydock release`)

## Deployment Files

By default, `wasm-drydock init` generates deployment files for Fly.io:

### `Dockerfile`

A multi-stage Docker build using cargo-chef for layer caching. Builds the frontend with `wasm-pack`, compiles SCSS, and produces a minimal production image with the backend binary.

### `.dockerignore`

Excludes unnecessary files from Docker build context:
- `target/` — Cargo build output
- `frontend/pkg/` — wasm-pack output
- `frontend/styles/screen.css` — compiled CSS
- `.git/` — version control
- `*.md` — documentation
- `fly.toml` — Fly.io config (not needed in image)

### `fly.toml`

Fly.io deployment configuration with sensible defaults:
- Internal port 3001
- HTTPS forced
- Auto-scaling (stop/start machines)
- `shared-cpu-1x` with 256MB memory

See [Fly.io deployment](../building-and-deployment/fly-io.md) for details.

### Skipping deployment files

Use `--no-deploy` to skip these files:

```bash
wasm-drydock init my-app --no-deploy
```
