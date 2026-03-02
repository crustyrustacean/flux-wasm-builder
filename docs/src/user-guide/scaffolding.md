# Project Scaffolding

The `init` command creates a fully-wired three-crate Cargo workspace ready for development.

## Command Syntax

```bash
flux-wasm-builder init <project-name> [options]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<project-name>` | Project name (used as directory name and crate prefix) |

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--path` | `-p` | `.` | Parent directory for the project |

## Generated Structure

```
my-app/
├── .cargo/
│   └── config.toml          # cargo alias: backend = "run -p my-app-backend"
├── .gitignore               # excludes target/ and frontend/pkg/
├── Cargo.toml               # workspace root
├── backend/
│   ├── Cargo.toml
│   ├── build.rs             # compile-time guard for embed-assets
│   └── src/
│       ├── main.rs          # server entry point
│       ├── build_subsystem/ # embedded build logic
│       │   ├── mod.rs
│       │   ├── build.rs
│       │   ├── build_coordinator.rs
│       │   ├── watcher.rs
│       │   ├── reload.rs
│       │   └── static_assets.rs
│       └── api/
│           └── mod.rs       # application route handlers
├── frontend/
│   ├── Cargo.toml
│   ├── index.html
│   └── src/
│       └── lib.rs           # Yew application
└── shared/
    ├── Cargo.toml
    └── src/
        └── lib.rs           # API types
```

## Workspace Configuration

### Root Cargo.toml

```toml
[workspace]
resolver = "3"
members = ["backend", "frontend", "shared"]

[workspace.package]
name = "my-app"
version = "0.1.0"
edition = "2024"
```

### Cargo Alias (.cargo/config.toml)

```toml
[alias]
backend = "run -p my-app-backend"
start = "run -p my-app-backend"
```

This allows running `cargo backend` from the workspace root.

## Crate Naming

Crate names are derived from the project name:

| Crate | Name Pattern | Example |
|-------|--------------|---------|
| Backend | `{name}-backend` | `my-app-backend` |
| Frontend | `{name}-frontend` | `my-app-frontend` |
| Shared | `{name}-shared` | `my-app-shared` |

## Environment Validation

Before creating files, `init` validates the environment:

1. **wasm-pack on PATH**: Checks if `wasm-pack` is available
2. **wasm-pack version**: Ensures version >= 0.13.0
3. **wasm32 target**: Verifies `wasm32-unknown-unknown` is installed

If any check fails, an actionable error message is displayed and no files are created.

## Error Cases

### Directory Already Exists

```bash
flux-wasm-builder init my-app
# If my-app/ already exists:
# error: directory 'my-app' already exists
```

### Missing Prerequisites

```bash
flux-wasm-builder init my-app
# If wasm-pack is not installed:
# error: wasm-pack not found on PATH — install from https://rustwasm.github.io/wasm-pack/
```

## Success Output

After successful scaffolding:

```
✓ Created project: my-app/

  my-app/
  ├── .cargo/config.toml
  ├── .gitignore
  ├── Cargo.toml
  ├── backend/
  ├── frontend/
  └── shared/

Next steps:
  cd my-app
  cargo backend
```

## Customizing the Template

The generated project is a starting point. After scaffolding, you can:

1. **Add API routes** in `backend/src/api/mod.rs`
2. **Modify frontend** in `frontend/src/lib.rs`
3. **Add shared types** in `shared/src/lib.rs`
4. **Configure the server** in `backend/src/main.rs`

The build subsystem in `backend/src/build_subsystem/` is copied from templates and owned by your project — modify it as needed.
