# Stage 0: Project Scaffolding

Stage 0 implements the `init` subcommand, which is the entry point for all new projects.

## Overview

**Command signature:**

```bash
flux-wasm-builder init <project-name>
```

**Key Behavior:**
- Validates environment (wasm-pack, wasm32 target)
- Creates a three-crate Cargo workspace
- Generates all necessary files from templates
- Provides actionable error messages

## Generated Structure

```
my-app/
├── .cargo/config.toml      # cargo alias
├── .gitignore
├── Cargo.toml              # workspace root
├── backend/
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── build_subsystem/
│       └── api/
├── frontend/
│   ├── Cargo.toml
│   ├── index.html
│   └── src/
└── shared/
    ├── Cargo.toml
    └── src/
```

## Implementation Components

### Environment Check

Before creating files, validates:

1. **wasm-pack on PATH**: Uses `which`/`where` command
2. **wasm-pack version**: Ensures >= 0.13.0
3. **wasm32 target**: Checks via `rustup target list --installed`

### Error Types

```rust
pub enum InitError {
    AlreadyExists(PathBuf),
    EnvCheck(#[from] EnvCheckError),
    CreateDir { path: PathBuf, source: std::io::Error },
    WriteFile { path: PathBuf, source: std::io::Error },
}
```

### Templates

All generated file contents are defined in `templates.rs` as functions or `&str` constants. This is the single source of truth for what a new project looks like.

## Key Files Generated

### Workspace Cargo.toml

```toml
[workspace]
resolver = "3"
members = ["backend", "frontend", "shared"]

[workspace.package]
name = "my-app"
version = "0.1.0"
edition = "2024"
```

### Cargo Config

```toml
[alias]
backend = "run -p my-app-backend"
start = "run -p my-app-backend"
```

### Backend build.rs

Validates `frontend/pkg/` exists when `embed-assets` feature is active:

```rust
fn main() {
    if std::env::var("CARGO_FEATURE_EMBED_ASSETS").is_ok() {
        let pkg = std::path::Path::new("../frontend/pkg");
        if !pkg.exists() || pkg.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            panic!("embedding assets requires frontend/pkg/ to exist...");
        }
    }
}
```

## Test Coverage

- Creates expected directory structure
- Workspace Cargo.toml lists all members
- Cargo config contains run alias
- .gitignore excludes target/ and pkg/
- Frontend Cargo.toml sets cdylib and pins Yew 0.21
- Backend main.rs uses `#[actix_web::main]`
- Backend build.rs uses env var, not cfg!()
- Fails if directory already exists
- Project name substituted into crate names

## Success Output

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
