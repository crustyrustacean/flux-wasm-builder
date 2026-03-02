# Stage 0 — Project Scaffolding (`init`)

**Implementation Plan for flux-wasm-builder**

---

## Overview

Stage 0 implements the `init` subcommand, which is the entry point for all new projects. This stage creates a fully wired, immediately runnable three-crate Cargo workspace with correct dependencies, stub source files, and a working `index.html`.

**Command signature:**

```
flux-wasm-builder init <project-name>
```

---

## Architecture

### Tool Structure (after Stage 0)

```
flux-wasm-builder/
├── Cargo.toml
├── src/
│   ├── main.rs                   # CLI entry point — clap parsing only
│   ├── lib.rs                    # public API of the library
│   ├── env_check.rs              # wasm-pack and rustup target verification
│   └── init/
│       ├── mod.rs                # scaffold logic
│       └── templates.rs          # all generated file contents as &str constants
└── tests/
    └── init.rs                   # integration tests
```

### Generated Project Layout

```
my-app/
├── .cargo/
│   └── config.toml               # workspace alias: run = "run -p backend"
├── .gitignore
├── Cargo.toml                    # workspace root
├── backend/
│   ├── Cargo.toml
│   ├── build.rs                  # compile-time guard for embed-assets feature
│   └── src/
│       ├── main.rs
│       ├── build_subsystem/
│       │   ├── mod.rs
│       │   ├── build.rs
│       │   ├── build_coordinator.rs
│       │   ├── watcher.rs
│       │   ├── reload.rs
│       │   └── static_assets.rs
│       └── api/
│           └── mod.rs            # application route handlers
├── frontend/
│   ├── Cargo.toml
│   ├── index.html
│   └── src/
│       └── lib.rs
└── shared/
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

---

## Implementation Steps

### Step 1: Project Setup and Dependencies

**File: `Cargo.toml`**

Add dependencies:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
tokio-test = "0.4"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Step 2: Error Types

**File: `src/init/mod.rs`**

Define `InitError` enum:

```rust
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("directory '{0}' already exists")]
    AlreadyExists(PathBuf),

    #[error("environment check failed: {0}")]
    EnvCheck(#[from] EnvCheckError),

    #[error("failed to create directory '{path}': {source}")]
    CreateDir { path: PathBuf, source: std::io::Error },

    #[error("failed to write '{path}': {source}")]
    WriteFile { path: PathBuf, source: std::io::Error },
}
```

**File: `src/env_check.rs`**

Define `EnvCheckError` enum:

```rust
#[derive(Debug, thiserror::Error)]
pub enum EnvCheckError {
    #[error("wasm-pack not found on PATH — install from https://rustwasm.github.io/wasm-pack/")]
    WasmPackNotFound,

    #[error("wasm-pack version {found} is below minimum required {required}")]
    WasmPackVersionTooOld { found: String, required: String },

    #[error("wasm32-unknown-unknown target not installed — run: rustup target add wasm32-unknown-unknown")]
    Wasm32TargetMissing,

    #[error("failed to invoke '{command}': {source}")]
    SpawnFailed { command: String, source: std::io::Error },

    #[error("failed to parse wasm-pack version output: {0}")]
    VersionParseFailed(String),
}
```

### Step 3: Environment Check Functions

**File: `src/env_check.rs`**

Implement three public functions:

1. **`wasm_pack_on_path() -> bool`**
   - Shells out to `which wasm-pack` (Unix) or `where wasm-pack` (Windows)
   - Returns `true` if found on PATH

2. **`wasm_pack_version_ok() -> Result<(), EnvCheckError>`**
   - Runs `wasm-pack --version`
   - Parses semver output (e.g., `wasm-pack 0.13.0`)
   - Returns `Ok(())` if version >= 0.13.0
   - Uses simple integer comparison, not full `semver` crate

3. **`wasm32_target_installed() -> Result<(), EnvCheckError>`**
   - Runs `rustup target list --installed`
   - Checks for `wasm32-unknown-unknown` in output

### Environment Check Behavior

When environment checks fail, the tool **prompts the user with installation instructions** and exits without creating any files:

| Check Failed                           | Error Message                                                                                       |
| -------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `wasm-pack` not on PATH                | `error: wasm-pack not found on PATH. Install it from https://rustwasm.github.io/wasm-pack/`         |
| `wasm-pack` version < 0.13.0           | `error: wasm-pack version {found} is below minimum required 0.13.0. Please upgrade wasm-pack.`      |
| `wasm32-unknown-unknown` not installed | `error: wasm32-unknown-unknown target not installed. Run: rustup target add wasm32-unknown-unknown` |

**Important:** The tool does **not** automatically install these dependencies. It provides clear, actionable instructions and exits with a non-zero status code, allowing the user to install them manually before re-running `init`.

**Helper function:**

```rust
fn version_meets_minimum(found: &str, minimum: &str) -> bool {
    fn parse(s: &str) -> Option<(u32, u32, u32)> {
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse::<u32>().unwrap_or(0);
        Some((major, minor, patch))
    }
    match (parse(found), parse(minimum)) {
        (Some(f), Some(m)) => f >= m,
        _ => false,
    }
}
```

### Step 4: CLI Entry Point

**File: `src/main.rs`**

```rust
use clap::{Parser, Subcommand};
use flux_wasm_builder::init::scaffold;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "flux-wasm-builder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new fullstack project
    Init {
        /// Project name (will be used as directory name)
        name: String,
        /// Parent directory (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
}

fn main() {
    init_tracing();

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init { name, path } => {
            scaffold(&path, &name)?;
        }
    }
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(false)
        .compact()
        .init();
}
```

### Step 5: Library Entry Point

**File: `src/lib.rs`**

```rust
pub mod env_check;
pub mod init;

// Re-exports for convenience
pub use init::{scaffold, InitError};
pub use env_check::EnvCheckError;
```

### Step 6: Scaffold Function

**File: `src/init/mod.rs`**

```rust
mod templates;

use std::path::Path;
use crate::env_check::{wasm_pack_on_path, wasm_pack_version_ok, wasm32_target_installed};

pub fn scaffold(root: &Path, name: &str) -> Result<(), InitError> {
    let span = tracing::info_span!("scaffold", project = name);
    let _enter = span.enter();

    // Step 1: Environment check
    tracing::info!("running environment checks");
    if !wasm_pack_on_path() {
        return Err(InitError::EnvCheck(EnvCheckError::WasmPackNotFound));
    }
    wasm_pack_version_ok()?;
    wasm32_target_installed()?;

    // Step 2: Existence check
    let project_root = root.join(name);
    if project_root.exists() {
        tracing::warn!(path = %project_root.display(), "target directory already exists");
        return Err(InitError::AlreadyExists(project_root));
    }

    // Step 3: Directory creation
    tracing::debug!("creating directory structure");
    create_dirs(&project_root)?;

    // Step 4: Write files
    tracing::debug!("writing project files");
    write_files(&project_root, name)?;

    // Step 5: Success output
    print_success(name);
    tracing::info!("project created successfully");
    Ok(())
}
```

### Step 7: Templates

**File: `src/init/templates.rs`**

All file contents as `&str` constants or functions that accept the project name:

| Template                    | Key Content                                                                                                                                                                                |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `WORKSPACE_CARGO_TOML`      | `members = ["backend", "frontend", "shared"]`, `resolver = "2"`                                                                                                                            |
| `CARGO_CONFIG`              | `[alias]\nrun = "run -p backend"`                                                                                                                                                          |
| `GITIGNORE`                 | `target/`, `frontend/pkg/`                                                                                                                                                                 |
| `backend_cargo_toml(name)`  | Dependencies: actix-web, actix-ws, tokio, serde_json, mime_guess, tracing, notify-debouncer-mini, thiserror, tracing-subscriber, shared (path). Feature: `embed-assets` with `include_dir` |
| `backend_build_rs`          | Reads `CARGO_FEATURE_EMBED_ASSETS` env var, panics if `frontend/pkg/` missing                                                                                                              |
| `backend_main_rs`           | Uses `#[actix_web::main]`, constructs `BuildConfig`, handles `AddrInUse`                                                                                                                   |
| `frontend_cargo_toml(name)` | `[lib] crate-type = ["cdylib"]`, yew 0.21 with `csr` feature, wasm-bindgen, gloo-net, shared                                                                                               |
| `frontend_lib_rs`           | Minimal Yew app with `#[wasm_bindgen(start)]`, fetches `/api/hello`                                                                                                                        |
| `frontend_index_html`       | Imports wasm-bindgen JS module as `type="module"`                                                                                                                                          |
| `shared_cargo_toml(name)`   | Only `serde = { version = "1", features = ["derive"] }`                                                                                                                                    |
| `shared_lib_rs`             | `HelloResponse` and `StatusResponse` structs, no `deny_unknown_fields`                                                                                                                     |

### Step 8: Unit Tests

**File: `src/init/mod.rs` (test module)**

| Test                                                           | Description                                |
| -------------------------------------------------------------- | ------------------------------------------ |
| `creates_expected_directory_structure`                         | Verifies all expected files exist          |
| `workspace_cargo_toml_lists_all_members_with_resolver_2`       | Checks workspace members and resolver      |
| `cargo_config_contains_run_alias_for_backend`                  | Checks `.cargo/config.toml`                |
| `gitignore_excludes_target_and_pkg`                            | Checks `.gitignore` content                |
| `frontend_cargo_toml_sets_cdylib_and_pins_yew_021`             | Checks frontend crate type and Yew version |
| `backend_main_uses_actix_web_main_not_tokio_main`              | Checks `#[actix_web::main]` usage          |
| `backend_build_rs_uses_env_var_not_cfg_macro`                  | Checks `CARGO_FEATURE_EMBED_ASSETS` usage  |
| `backend_cargo_toml_has_embed_assets_feature_with_include_dir` | Checks feature definition                  |
| `shared_lib_does_not_use_deny_unknown_fields`                  | Ensures forward compatibility              |
| `fails_if_directory_already_exists`                            | Tests `InitError::AlreadyExists`           |
| `project_name_substituted_into_crate_names`                    | Tests name substitution                    |

### Step 9: Integration Tests

**File: `tests/init.rs`**

| Test                                            | Description                  |
| ----------------------------------------------- | ---------------------------- |
| `init_creates_project_in_current_directory`     | End-to-end project creation  |
| `init_prints_success_message_with_project_name` | Verifies success output      |
| `init_prints_cargo_run_next_step`               | Verifies "Next steps" output |
| `init_fails_clearly_if_directory_exists`        | Tests error handling         |
| `init_requires_name_argument`                   | Tests CLI validation         |

---

## Test-Driven Development Workflow

Following the spec's TDD approach:

1. **Write tests first** — Create all unit and integration tests
2. **Confirm tests fail** — Run `cargo test` and verify failures
3. **Implement** — Write code until all tests pass
4. **Manual verification** — Run `cargo run -- init test-project` and verify output

---

## Acceptance Criteria

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] `flux-wasm-builder init hello-world` produces the complete structure
- [ ] Running `flux-wasm-builder init hello-world` a second time prints a clear error and exits non-zero
- [ ] `cargo check` succeeds in the generated workspace (host target only)
- [ ] `cargo run` in the generated workspace root resolves to the backend binary via `.cargo/config.toml` alias

---

## Dependencies Summary

### Tool Dependencies

| Crate                | Purpose                 |
| -------------------- | ----------------------- |
| `clap`               | CLI argument parsing    |
| `thiserror`          | Error derive macros     |
| `tracing`            | Structured logging      |
| `tracing-subscriber` | Logging initialization  |
| `tempfile`           | Test directory creation |
| `assert_cmd`         | CLI integration testing |
| `predicates`         | Test assertions         |

### Generated Backend Dependencies

| Crate                   | Purpose                         |
| ----------------------- | ------------------------------- |
| `actix-web`             | HTTP server                     |
| `actix-ws`              | WebSocket handling              |
| `tokio`                 | Async runtime                   |
| `serde_json`            | JSON serialization              |
| `mime_guess`            | MIME type inference             |
| `tracing`               | Structured logging              |
| `tracing-subscriber`    | Logging initialization          |
| `notify-debouncer-mini` | File watching                   |
| `include_dir`           | Asset embedding (feature-gated) |
| `thiserror`             | Error derive macros             |

### Generated Frontend Dependencies

| Crate          | Purpose                                  |
| -------------- | ---------------------------------------- |
| `yew`          | Frontend framework (0.21, `csr` feature) |
| `wasm-bindgen` | Rust↔JS FFI                              |
| `gloo-net`     | HTTP client for WASM                     |

### Generated Shared Dependencies

| Crate   | Purpose                          |
| ------- | -------------------------------- |
| `serde` | Serialization (`derive` feature) |

---

## File Checklist

### Tool Files to Create

- [ ] `src/main.rs` — CLI entry point
- [ ] `src/lib.rs` — Library entry point
- [ ] `src/env_check.rs` — Environment verification
- [ ] `src/init/mod.rs` — Scaffold logic and error types
- [ ] `src/init/templates.rs` — File content templates
- [ ] `tests/init.rs` — Integration tests

### Generated Files (Templates)

- [ ] `<n>/Cargo.toml` — Workspace root
- [ ] `<n>/.cargo/config.toml` — Cargo alias
- [ ] `<n>/.gitignore` — Git ignore rules
- [ ] `<n>/backend/Cargo.toml` — Backend dependencies
- [ ] `<n>/backend/build.rs` — Build script
- [ ] `<n>/backend/src/main.rs` — Backend entry point
- [ ] `<n>/backend/src/build_subsystem/mod.rs` — Build subsystem module
- [ ] `<n>/backend/src/build_subsystem/build.rs` — Stub file
- [ ] `<n>/backend/src/build_subsystem/build_coordinator.rs` — Stub file
- [ ] `<n>/backend/src/build_subsystem/watcher.rs` — Stub file
- [ ] `<n>/backend/src/build_subsystem/reload.rs` — Stub file
- [ ] `<n>/backend/src/build_subsystem/static_assets.rs` — Stub file
- [ ] `<n>/backend/src/api/mod.rs` — API routes stub
- [ ] `<n>/frontend/Cargo.toml` — Frontend dependencies
- [ ] `<n>/frontend/index.html` — HTML entry point
- [ ] `<n>/frontend/src/lib.rs` — Frontend entry point
- [ ] `<n>/shared/Cargo.toml` — Shared dependencies
- [ ] `<n>/shared/src/lib.rs` — Shared types

---

## Notes

1. **No `deny_unknown_fields`** — The shared types must not use `#[serde(deny_unknown_fields)]` for forward compatibility (see spec Section 5.2)

2. **`CARGO_FEATURE_EMBED_ASSETS`** — The build script must use the environment variable, not `cfg!()`, for feature detection

3. **`#[actix_web::main]`** — The backend main must use this attribute, not `#[tokio::main]`

4. **Cross-platform PATH check** — Use `which` on Unix and `where` on Windows for checking if `wasm-pack` is on PATH

5. **Stub files** — Build subsystem files are empty stubs in Stage 0, to be implemented in later stages
