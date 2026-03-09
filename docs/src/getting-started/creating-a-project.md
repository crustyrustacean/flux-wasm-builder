# Creating a Project

## Scaffold a new project

```bash
wasm-drydock init my-app
cd my-app
```

`wasm-drydock init` validates your environment before scaffolding — it checks that `wasm-pack` is installed, meets the minimum version requirement, and that the `wasm32-unknown-unknown` target is present. If any check fails, it reports the problem and exits cleanly.

The generated project is a three-crate Cargo workspace with deployment files for Fly.io:

```
my-app/
├── .cargo/config.toml
├── .dockerignore
├── .gitignore
├── Cargo.toml
├── Dockerfile
├── drydock.toml
├── fly.toml
├── backend/
├── frontend/
└── shared/
```

See [Project Structure](../project-structure/README.md) for a detailed walkthrough of what each crate contains and why.

## Skip deployment files

If you don't need Fly.io deployment configuration, use the `--no-deploy` flag:

```bash
wasm-drydock init my-app --no-deploy
```

This skips generating `Dockerfile`, `.dockerignore`, and `fly.toml`.

## Project name rules

Project names follow Cargo conventions:

- 1–64 characters
- Alphanumeric characters, hyphens, and underscores only
- Cannot start with a hyphen or underscore
- Cannot be a Rust keyword or reserved Cargo command name
