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
