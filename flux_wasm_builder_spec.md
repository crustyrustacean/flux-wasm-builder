# Integrated Fullstack Build Tool

**Design Specification — Actix-web + Yew**

Personal project · Draft 0.4 · March 2026

---

## 1. Problem Statement

Building a fullstack web application with Actix-web on the backend and Yew on the frontend currently requires two parallel, independent build pipelines that have no awareness of each other. The conventional approach involves:

- Running `trunk serve` in one terminal to watch and compile the Yew frontend, serving assets on a dedicated port (e.g. 8080).
- Running `cargo run` in a second terminal to start the Actix-web backend on a different port (e.g. 8000).
- Configuring a reverse proxy — either inside trunk's `Trunk.toml` or via an external tool — to forward API requests from the frontend port to the backend port.
- Managing a Cargo workspace that typically contains at least two crates: one for the frontend and one for the backend, each with separate build concerns.

This arrangement compounds in several ways as a project grows:

- Two watch processes compete for the same source tree, causing redundant recompilation and confusing terminal output.
- The proxy configuration is a third artifact to maintain and keep consistent with both the frontend and backend routing.
- Shared data types — the structs and enums that define the API contract — must be duplicated or extracted into a third shared crate, requiring explicit dependency wiring.
- Deployment involves coordinating two build artifacts, two sets of environment variables, and often a separate static file hosting step.
- `trunk` itself has seen declining maintenance activity, introducing long-term sustainability risk for any project that depends on it as a primary build tool.

The goal of this project is to eliminate this dual-pipeline complexity for personal fullstack Rust projects. A single `cargo run` should be sufficient to build, serve, and watch the entire application — frontend and backend — in development, and a single `cargo build --release --features embed-assets` should produce a self-contained binary that serves both the API and the compiled WebAssembly assets.

---

## 2. Goals and Non-Goals

### 2.1 Goals

- **Single entry point:** `cargo run` in the workspace root of a generated project starts everything in development mode.
- **Single binary deployment:** `cargo build --release --features embed-assets` produces one binary that serves the full application.
- **Inline compiler errors:** Rust compilation errors from both the frontend and backend crates appear in the same terminal, formatted consistently.
- **Live reload:** A saved change to any frontend source file triggers an incremental `wasm-pack` rebuild and signals the browser to refresh without manually restarting the server.
- **Shared types crate:** A common crate for API-boundary types (request/response structs) is a first-class part of the project layout, with `serde` derives for serialization in both directions.
- **No JavaScript build tooling required:** No Node.js, npm, Rollup, Webpack, or Vite in the critical path. The tool's only external dependencies are `wasm-pack` and the Rust toolchain.
- **Self-contained build subsystem:** The build subsystem logic lives inside the generated backend crate itself — copied from the tool's templates at init time. The tool generates the code; it does not act as a runtime dependency of generated projects.
- **Project scaffolding:** An `init` command creates a fully wired, immediately runnable three-crate workspace with correct dependencies, stub source files, and a working `index.html` — no manual setup required.

### 2.2 Non-Goals

- **General-purpose frontend framework support:** This tool targets Yew specifically. No React, Svelte, or other JS framework support is planned.
- **Multi-target builds:** A single frontend crate targeting a single WASM binary is the expected configuration. Multi-page apps or micro-frontend architectures are out of scope.
- **Public release or crates.io publication:** This is a personal productivity tool. API stability, semver, and documentation conventions appropriate for published crates are not required initially.
- **CSS/SASS preprocessing:** Static CSS files are served as-is. A SASS compilation step is not planned for the initial version.
- **npm ecosystem integration:** The tool does not consume or produce npm packages. `wasm-pack`'s `--target web` output is used directly.

---

## 3. Architecture Overview

### 3.1 The Tool's Own Structure

The tool (`my-tool`) is itself a single Cargo crate with both a binary and a library target:

```
my-tool/
├── Cargo.toml
├── src/
│   ├── main.rs                   # CLI entry point — clap parsing only, delegates to lib
│   ├── lib.rs                    # public API of the build subsystem
│   ├── init/
│   │   ├── mod.rs                # scaffold logic: creates dirs and writes files
│   │   └── templates.rs          # all generated file contents as &str constants
│   ├── build.rs                  # wasm-pack subprocess invocation
│   ├── build_coordinator.rs      # serialized rebuild loop
│   ├── watcher.rs                # notify-debouncer-mini wrapper
│   ├── reload.rs                 # WebSocket broadcast and script injection
│   ├── static_assets.rs          # Actix handlers for pkg/ and index.html
│   └── env_check.rs              # wasm-pack and rustup target verification
└── tests/
    ├── init.rs
    ├── build.rs
    ├── serve.rs
    ├── watcher.rs
    ├── reload.rs
    ├── embed.rs
    └── api.rs
```

The library target exposes the build subsystem so that it can be unit-tested in isolation without subprocess overhead. The binary target is a thin shell that parses CLI arguments with `clap` and calls into the library. The tool does **not** become a runtime dependency of generated projects — it generates source code that the project owns directly.

### 3.2 Generated Project Layout

The `init` command produces a Cargo workspace with three member crates. The generated `backend` crate contains the build subsystem source inline — copied from the tool's templates at init time — and owns it outright. There is no `path` or registry dependency back to the tool itself.

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

### 3.3 Runtime Modes

The backend binary operates in two distinct modes, selected at compile time via a Cargo feature flag:

| Mode                                  | Behaviour                                                                                                                                                                                                                                                                    |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dev` (default)                       | On startup, invokes `wasm-pack` to build the frontend crate, then starts the Actix-web server. A file watcher monitors the frontend source tree. On detected changes, the build pipeline is re-invoked and connected browsers are signalled to reload via WebSocket.         |
| `release` (`--features embed-assets`) | The compiled WebAssembly binary and JS glue from `./pkg/` are embedded directly into the backend binary at compile time using `include_dir`. No `wasm-pack` invocation occurs at runtime. No file watcher is started. The result is a zero-dependency self-contained binary. |

### 3.4 Request Routing

The Actix-web server handles three categories of requests, matched in this order:

- **API routes:** Any path registered by the application (e.g. `/api/...`) is handled by the normal Actix handler chain. These are registered first and always take precedence.
- **Static asset routes:** Requests for files under `/pkg/` are served by the build subsystem's asset handler. In dev mode files are read from `frontend/pkg/` on each request. In release mode they are served from embedded bytes. The `.wasm` file is served with `Content-Type: application/wasm` to enable browser streaming instantiation.
- **SPA fallback:** All unmatched routes return `index.html`, enabling client-side routing within Yew. This is registered as Actix's `default_service`, not as a regular route, to ensure it cannot shadow `/pkg/` or `/api/` paths.

### 3.5 Component Diagram

Data flow in dev mode at startup:

```
cargo run          (resolved via .cargo/config.toml alias to -p backend)
  └─► backend/src/main.rs  [#[actix_web::main]]
        ├─► env_check: wasm-pack >= 0.13.0 on PATH, wasm32 target installed
        ├─► BuildSubsystem::start()
        │     ├─► wasm-pack build frontend/ --target web --dev
        │     │     (subprocess, stdout+stderr inherited, timeout 300s)
        │     ├─► notify_debouncer: watches frontend/src/**/*.rs  (300ms quiet period)
        │     └─► tokio::spawn: build_coordinator loop
        │               on signal → rebuild → broadcast::send(())
        └─► actix_web::HttpServer::new()
              ├─► /api/...          (app handlers — registered first)
              ├─► /pkg/{filename}   (wasm + JS glue assets)
              ├─► /ws/reload        (WebSocket — dev mode only, cfg gated)
              └─► default_service   (SPA fallback → index.html + injected reload script)
```

---

## 4. Build Subsystem Design

### 4.1 wasm-pack Invocation

The frontend compilation is performed by shelling out to `wasm-pack`. The subsystem does not link against `wasm-bindgen-cli-support` directly, to avoid the significant version-coupling friction that approach introduces. The invocation is:

```
wasm-pack build <frontend_crate_path> --target web [--dev | --release]
```

stdout and stderr from the subprocess are streamed directly to the parent process's stderr in real time so that Rust compiler diagnostics appear inline without buffering. The subsystem treats a non-zero exit code from `wasm-pack` as a build failure; a failed build does not restart the server or trigger a browser reload. The previously-served assets remain in place until a successful build replaces them.

The subprocess invocation is wrapped with `tokio::time::timeout` with a configurable limit (default 300 seconds). If the timeout is exceeded, the child process is killed and `BuildError::Timeout` is returned. This prevents a hung `wasm-pack` — e.g. due to a network stall during dependency resolution on a cold cache — from blocking server startup indefinitely.

### 4.2 File Watching

The `notify-debouncer-mini` crate is used for filesystem event watching with built-in debouncing. It is a purpose-built companion to `notify` that manages the debounce interval internally, avoiding the need to implement a separate timer. The debounce quiet period defaults to 300ms and is configurable via `BuildConfig`.

The watcher monitors the frontend crate's `src/` directory recursively. Only events on files with a `.rs` extension trigger a rebuild; changes to `Cargo.toml`, `index.html`, or other files are ignored in the initial version. VSCodium and Neovim both use atomic save strategies that briefly create and replace temporary files — the debounce window absorbs these and produces a single rebuild trigger per logical save.

**Watcher lifetime:** The `Debouncer` value returned by `notify_debouncer_mini::new_debouncer` must be kept alive for the entire duration of the server process. It must be stored in a long-lived struct, never in a local variable that would be dropped at the end of a function scope. Dropping the debouncer silently stops all file watching with no error or warning.

### 4.3 Live Reload

Live reload is implemented via a WebSocket connection from the browser to `/ws/reload`. The `index.html` response in dev mode has the following script injected immediately before the closing `</body>` tag at serve time. The script is **not** present in the `index.html` file on disk.

```javascript
<script>
(function() {
  function connect() {
    const ws = new WebSocket('ws://' + location.host + '/ws/reload');
    ws.onmessage = () => location.reload();
    ws.onclose = () => setTimeout(connect, 1000);
  }
  connect();
})();
</script>
```

The `ws.onclose` reconnect loop means the browser automatically re-establishes the connection if the server restarts (e.g. after a manual backend restart), and reloads the page on the next successful build.

After a successful rebuild, the build coordinator calls `let _ = broadcast_tx.send(())`. The `let _` is intentional: `send` returns `Err` when there are no active subscribers, which is expected when no browser tabs are open. This must not be treated as an error.

### 4.4 Release Mode Asset Embedding

In release mode (the `embed-assets` Cargo feature), the contents of `frontend/pkg/` and `frontend/index.html` are embedded into the binary at compile time using `include_dir` and `include_bytes!`. The dev-mode handlers, file watcher, and WebSocket endpoint are all compiled out via `#[cfg(not(feature = "embed-assets"))]`.

The `backend/build.rs` script detects the feature via the `CARGO_FEATURE_EMBED_ASSETS` environment variable — which Cargo sets automatically when the feature is active. It does **not** use `cfg!()`, which does not work for feature detection in `build.rs`. If `frontend/pkg/` is absent or empty, `build.rs` panics with a clear, human-readable error directing the developer to run `wasm-pack` first.

The release build sequence is:

```bash
wasm-pack build frontend/ --target web --release
cargo build --release --features embed-assets
```

---

## 5. Shared Types Design

### 5.1 Purpose

The `shared` crate enforces the API contract at the Rust type system level. Any struct or enum that crosses the HTTP boundary is defined here. Both the backend and frontend depend on this crate, meaning a mismatch between what the backend serializes and what the frontend expects to deserialize is a compile error rather than a runtime panic.

### 5.2 Constraints

The `shared` crate must:

- Depend only on `serde` with the `derive` feature.
- Compile cleanly for both `wasm32-unknown-unknown` (frontend) and the host target (backend).
- Never pull in `web-sys`, `wasm-bindgen`, `actix-web`, or any crate that does not support the WASM target.

The `shared` crate must **not** use `#[serde(deny_unknown_fields)]` on any type. This is a deliberate forward-compatibility decision: as the backend adds new fields to response types over time, an older compiled frontend will safely ignore the unknown fields rather than panicking at runtime. This rule must be followed for all types added to `shared` in the future.

### 5.3 Stub Types

The generated `shared/src/lib.rs` provides two example types to establish the pattern:

```rust
use serde::{Deserialize, Serialize};

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

---

## 6. Configuration

The build subsystem is configured via a `BuildConfig` struct populated in `backend/src/main.rs`. No external config file is used; all configuration is Rust code.

```rust
BuildConfig {
    frontend_crate_path: PathBuf,  // e.g. "../frontend"
    pkg_output_path: PathBuf,      // default: frontend_crate_path + "/pkg"
    index_html_path: PathBuf,      // default: frontend_crate_path + "/index.html"
    watch_debounce_ms: u64,        // default: 300
    reload_ws_path: String,        // default: "/ws/reload"
    port: u16,                     // default: 8080
    build_timeout_secs: u64,       // default: 300
}
```

Reasonable defaults are provided for all fields via a builder pattern. `port` is a first-class field from day one to avoid a future breaking change when port configuration is needed. If the requested port is already bound, the generated `main.rs` catches the `AddrInUse` bind error and prints:

```
error: port 8080 is already in use. Change the port in BuildConfig.
```

---

## 7. Key Dependencies

Dependencies are split by where they are used. The tool's own crate and the generated project backend share similar dependencies, but `clap` belongs only to the tool.

### 7.1 Tool (`my-tool`) Dependencies

| Crate        | Purpose                                                           |
| ------------ | ----------------------------------------------------------------- |
| `clap`       | CLI argument parsing for `init` and any future subcommands        |
| `tempfile`   | Creating isolated scratch directories in tests                    |
| `assert_cmd` | Invoking the compiled binary as a subprocess in integration tests |
| `predicates` | Fluent assertions on command output in integration tests          |

### 7.2 Generated Backend Dependencies

| Crate                   | Purpose                                                                    |
| ----------------------- | -------------------------------------------------------------------------- |
| `actix-web`             | HTTP server, routing, static file serving                                  |
| `actix-ws`              | WebSocket handling (preferred over actix-web built-in for async broadcast) |
| `tokio`                 | Async runtime; process spawning, channels, task management                 |
| `notify-debouncer-mini` | Filesystem event watching with built-in debouncing                         |
| `include_dir`           | Compile-time directory embedding (behind `embed-assets` feature)           |
| `serde_json`            | JSON serialization for API responses                                       |
| `mime_guess`            | MIME type inference for static file serving                                |
| `tracing`               | Structured logging throughout the build subsystem                          |

### 7.3 Generated Frontend Dependencies

| Crate                      | Purpose                                                                                  |
| -------------------------- | ---------------------------------------------------------------------------------------- |
| `yew`                      | Frontend component framework (version **0.21**, `csr` feature)                           |
| `wasm-bindgen`             | Rust↔JS FFI glue                                                                         |
| `gloo-net`                 | Async HTTP client for the WASM target                                                    |
| `web-sys`                  | DOM API bindings (`Window` and `Document` features only; add further features as needed) |
| `wasm_logger`              | Routes Rust `log` calls to the browser console                                           |
| `log`                      | Logging facade (`log::info!`, `log::error!`, etc.)                                       |
| `console_error_panic_hook` | Prints Rust panics to the browser console instead of silently aborting                   |

### 7.4 Generated Shared Dependencies

| Crate   | Purpose                                         |
| ------- | ----------------------------------------------- |
| `serde` | Serialization framework (`derive` feature only) |

### 7.5 Workspace-Level Test Dependencies

```toml
[dev-dependencies]
assert_cmd = "2"
tempfile = "3"
predicates = "3"
tokio-test = "0.4"
```

---

## 8. Implementation Stages

Development proceeds in self-contained stages. Each stage ends with a usable, runnable tool. Each stage follows a strict TDD cycle: write the tests first, confirm they fail, implement until they pass, then move to the next stage. No stage is considered complete until all its tests pass and the tool can be exercised manually end-to-end.

---

### Stage 0 — Project Scaffolding (`init`)

#### Overview

Implement the `init` subcommand. This is the entry point for all new projects and the stage that makes every subsequent stage easier to develop and test.

The command signature is:

```
my-tool init <project-name>
```

#### Implementation Plan

1. Create `src/main.rs` with a `clap::Command` that has a single `init` subcommand accepting a positional `<n>` argument. All logic delegates to the library.

2. Create `src/env_check.rs` with three public functions:
   - `wasm_pack_on_path() -> bool` — shells out to `which wasm-pack`.
   - `wasm_pack_version_ok() -> bool` — runs `wasm-pack --version`, parses the semver output, and returns `true` if the version is >= **0.13.0** (the minimum required version).
   - `wasm32_target_installed() -> bool` — runs `rustup target list --installed` and checks for `wasm32-unknown-unknown` in the output.

3. Create `src/init/mod.rs` with a public `scaffold(root: &Path, name: &str) -> Result<(), InitError>` function. The function accepts the parent directory and project name, performs all file creation, and is directly unit-testable without a real working directory.

4. Create `src/init/templates.rs` containing all generated file contents as functions or `&str` constants. This is the single source of truth for what a new project looks like.

5. The `scaffold` function performs these steps in order, failing fast with a descriptive `InitError` variant on any error:
   - **Step 1: Environment check.** Call all three `env_check` functions. If `wasm-pack` is absent, print: `error: wasm-pack not found on PATH. Install it from https://rustwasm.github.io/wasm-pack/`. If the version is below 0.13.0, print the found version and the minimum required. If the wasm32 target is missing, print: `error: wasm32-unknown-unknown target not installed. Run: rustup target add wasm32-unknown-unknown`. Exit without creating any files or directories.

   - **Step 2: Existence check.** If `root/name/` already exists, return `InitError::AlreadyExists`. Do not create a partial project.

   - **Step 3: Directory creation.** Create all required directories: `<n>/`, `<n>/.cargo/`, `<n>/backend/src/build_subsystem/`, `<n>/backend/src/api/`, `<n>/frontend/src/`, `<n>/shared/src/`.

   - **Step 4: Write all files** from `templates.rs`. The project name is substituted into `Cargo.toml` `name` fields as `<n>-backend`, `<n>-frontend`, and `<n>-shared`.

   - **Step 5: Print success summary.**

6. Key properties of generated files:
   - **`<n>/Cargo.toml`** — workspace root with `members = ["backend", "frontend", "shared"]`,
     `resolver = "3"`, and `edition = "2024"` in `[workspace.package]`.
   - **`<n>/.cargo/config.toml`** — contains `[alias]\nrun = "run -p backend"`. This is required: without it, `cargo run` in a workspace with multiple crates errors rather than selecting the backend binary.
   - **`<n>/.gitignore`** — ignores `target/` and `frontend/pkg/`. Without this, the first `git status` after a build is overwhelmed with build artifacts.
   - **`<n>/backend/Cargo.toml`** — depends on `actix-web`, `actix-ws`, `tokio` (features = ["full"]), `serde_json`, `mime_guess`, `tracing`, `notify-debouncer-mini`, and `shared` (path = "../shared"). Includes `include_dir` behind `[features] embed-assets = ["dep:include_dir"]`.
   - **`<n>/backend/build.rs`** — reads `CARGO_FEATURE_EMBED_ASSETS` env var (not `cfg!()`) to detect the feature; asserts `../frontend/pkg/` exists and is non-empty if active.
   - **`<n>/backend/src/main.rs`** — uses `#[actix_web::main]`, **not** `#[tokio::main]`. Constructs `BuildConfig` with defaults, calls `BuildSubsystem::start().await`, then binds the Actix server. Catches `AddrInUse` errors and prints an actionable message.
   - **`<n>/frontend/Cargo.toml`** — `[lib] crate-type = ["cdylib"]`. `edition = "2024"`.
     Depends on `yew = { version = "0.21", features = ["csr"] }`, `wasm-bindgen`, `gloo-net`,
     `web-sys = { version = "0.3", features = ["Window", "Document"] }`,
     `wasm_logger`, `log`, `console_error_panic_hook`, and `shared` (path = "../shared").
   - **`<n>/frontend/src/lib.rs`** — a minimal Yew 0.21 app with a `#[wasm_bindgen(start)]` entry point and a single `App` function component that fetches `/api/hello` and displays the response.
   - **`<n>/frontend/index.html`** — a minimal HTML file that imports the wasm-bindgen JS module as `type="module"`. Contains no reference to the live reload script.
   - **`<n>/shared/Cargo.toml`** — depends only on `serde = { version = "1", features = ["derive"] }`.
   - **`<n>/shared/src/lib.rs`** — defines `HelloResponse` and `StatusResponse` as in Section 5.3. Does **not** use `#[serde(deny_unknown_fields)]`.

7. **Success output:**

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
  cargo run
```

#### Test Plan

**Unit tests** — in `src/init/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_expected_directory_structure() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();

        assert!(root.path().join("my-app/Cargo.toml").exists());
        assert!(root.path().join("my-app/.cargo/config.toml").exists());
        assert!(root.path().join("my-app/.gitignore").exists());
        assert!(root.path().join("my-app/backend/Cargo.toml").exists());
        assert!(root.path().join("my-app/backend/build.rs").exists());
        assert!(root.path().join("my-app/backend/src/main.rs").exists());
        assert!(root.path().join("my-app/frontend/Cargo.toml").exists());
        assert!(root.path().join("my-app/frontend/src/lib.rs").exists());
        assert!(root.path().join("my-app/frontend/index.html").exists());
        assert!(root.path().join("my-app/shared/Cargo.toml").exists());
        assert!(root.path().join("my-app/shared/src/lib.rs").exists());
    }

    #[test]
    fn workspace_cargo_toml_lists_all_members_with_resolver_2() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/Cargo.toml")
        ).unwrap();
        assert!(contents.contains("backend"));
        assert!(contents.contains("frontend"));
        assert!(contents.contains("shared"));
        assert!(contents.contains("resolver = \"2\""));
    }

    #[test]
    fn cargo_config_contains_run_alias_for_backend() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/.cargo/config.toml")
        ).unwrap();
        assert!(contents.contains("[alias]"));
        assert!(contents.contains("-p backend"));
    }

    #[test]
    fn gitignore_excludes_target_and_pkg() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/.gitignore")
        ).unwrap();
        assert!(contents.contains("target/"));
        assert!(contents.contains("frontend/pkg/"));
    }

    #[test]
    fn frontend_cargo_toml_sets_cdylib_and_pins_yew_021() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/frontend/Cargo.toml")
        ).unwrap();
        assert!(contents.contains("cdylib"));
        assert!(contents.contains("yew"));
        assert!(contents.contains("0.21"));
    }

    #[test]
    fn backend_main_uses_actix_web_main_not_tokio_main() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/backend/src/main.rs")
        ).unwrap();
        assert!(contents.contains("actix_web::main"));
        assert!(!contents.contains("tokio::main"));
    }

    #[test]
    fn backend_build_rs_uses_env_var_not_cfg_macro() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/backend/build.rs")
        ).unwrap();
        assert!(contents.contains("CARGO_FEATURE_EMBED_ASSETS"));
        // cfg!() does not work for feature detection in build.rs
        assert!(!contents.contains("cfg!(feature"));
    }

    #[test]
    fn backend_cargo_toml_has_embed_assets_feature_with_include_dir() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/backend/Cargo.toml")
        ).unwrap();
        assert!(contents.contains("embed-assets"));
        assert!(contents.contains("include_dir"));
    }

    #[test]
    fn shared_lib_does_not_use_deny_unknown_fields() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "my-app").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("my-app/shared/src/lib.rs")
        ).unwrap();
        assert!(contents.contains("HelloResponse"));
        assert!(contents.contains("Serialize"));
        assert!(contents.contains("Deserialize"));
        assert!(!contents.contains("deny_unknown_fields"),
            "shared types must not use deny_unknown_fields — see Section 5.2");
    }

    #[test]
    fn fails_if_directory_already_exists() {
        let root = tempdir().unwrap();
        std::fs::create_dir(root.path().join("my-app")).unwrap();
        let result = scaffold(root.path(), "my-app");
        assert!(matches!(result, Err(InitError::AlreadyExists)));
    }

    #[test]
    fn project_name_substituted_into_crate_names() {
        let root = tempdir().unwrap();
        scaffold(root.path(), "cool-project").unwrap();
        let contents = std::fs::read_to_string(
            root.path().join("cool-project/backend/Cargo.toml")
        ).unwrap();
        assert!(contents.contains("cool-project"));
    }
}
```

**Integration tests** — in `tests/init.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn init_creates_project_in_current_directory() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("my-tool").unwrap()
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("test-project/Cargo.toml").exists());
    assert!(dir.path().join("test-project/.cargo/config.toml").exists());
    assert!(dir.path().join("test-project/.gitignore").exists());
}

#[test]
fn init_prints_success_message_with_project_name() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("my-tool").unwrap()
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("test-project"));
}

#[test]
fn init_prints_cargo_run_next_step() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("my-tool").unwrap()
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(contains("cargo run"));
}

#[test]
fn init_fails_clearly_if_directory_exists() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("test-project")).unwrap();
    Command::cargo_bin("my-tool").unwrap()
        .args(["init", "test-project"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(contains("already exists"));
}

#[test]
fn init_requires_name_argument() {
    Command::cargo_bin("my-tool").unwrap()
        .args(["init"])
        .assert()
        .failure();
}
```

#### Acceptance Criteria

- All unit tests pass.
- All integration tests pass.
- `my-tool init hello-world` produces the complete structure described in Section 3.2.
- `my-tool init hello-world` a second time prints a clear error and exits non-zero without modifying the existing project.
- `cargo check` succeeds in the generated workspace (host target only — no WASM compilation needed at this stage).
- `cargo run` in the generated workspace root resolves to the backend binary via the `.cargo/config.toml` alias without manual `-p backend`.

---

### Stage 1 — Static Build Pipeline

#### Overview

Implement the build subsystem's core function: invoking `wasm-pack` as a subprocess, streaming its output to the terminal in real time, and returning a typed `Result`. Wire this into `backend/src/main.rs` so that `cargo run` triggers a frontend build before the Actix server binds.

**First-run note:** The first `wasm-pack` invocation on a fresh project downloads the entire Yew dependency tree, which may take several minutes on a cold `cargo` cache. This is expected behaviour, not a hang. The 300-second default timeout is intentionally generous to accommodate this. Subsequent builds are fast due to incremental compilation.

#### Implementation Plan

- Implement `run_wasm_pack(config: &BuildConfig) -> Result<(), BuildError>` in the build subsystem.
- Use `tokio::process::Command` with `stdout(Stdio::inherit())` and `stderr(Stdio::inherit())`.
- Wrap `child.wait()` in `tokio::time::timeout(Duration::from_secs(config.build_timeout_secs), ...)`. On timeout, call `child.kill().await` before returning `BuildError::Timeout`.
- For testability, also expose `run_wasm_pack_with_env(config: &BuildConfig, extra_env: &[(&str, &str)]) -> Result<(), BuildError>` — identical to `run_wasm_pack` but applies additional environment variable overrides to the subprocess. This seam allows tests to shadow `PATH` without affecting the system environment.
- Expose `run_command_with_timeout(cmd: Command, timeout: Duration) -> Result<(), BuildError>` as a testable primitive so the timeout behaviour can be tested with a `sleep` command rather than requiring a real `wasm-pack` failure.
- In `main.rs`, call `BuildSubsystem::start().await` before `HttpServer::new(...)`. A build failure at startup exits the process with a non-zero code. The server is not started if the initial build fails.

#### Test Plan

**Unit tests** — in the build subsystem's `build.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn build_config_default_pkg_path_is_relative_to_frontend() {
        let config = BuildConfig::new("../frontend");
        assert_eq!(config.pkg_output_path, PathBuf::from("../frontend/pkg"));
    }

    #[test]
    fn build_config_default_index_html_path_is_relative_to_frontend() {
        let config = BuildConfig::new("../frontend");
        assert_eq!(config.index_html_path, PathBuf::from("../frontend/index.html"));
    }

    #[tokio::test]
    async fn returns_error_when_wasm_pack_not_on_path() {
        let config = BuildConfig::new("/nonexistent");
        let result = run_wasm_pack_with_env(&config, &[("PATH", "")]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn returns_wasm_pack_failed_for_invalid_crate_path() {
        let dir = tempdir().unwrap();
        let config = BuildConfig::new(dir.path());
        let result = run_wasm_pack(&config).await;
        assert!(matches!(result, Err(BuildError::WasmPackFailed { .. })));
    }

    #[tokio::test]
    async fn returns_timeout_error_when_process_hangs() {
        let mut cmd = tokio::process::Command::new("sleep");
        cmd.arg("60");
        let result = run_command_with_timeout(
            cmd,
            std::time::Duration::from_millis(100)
        ).await;
        assert!(matches!(result, Err(BuildError::Timeout)));
    }
}
```

**Integration tests** — in `tests/build.rs`:

```rust
// Run with: RUN_WASM_TESTS=1 cargo test --test build

fn wasm_tests_enabled() -> bool {
    std::env::var("RUN_WASM_TESTS").is_ok()
}

#[test]
fn wasm_pack_builds_scaffolded_frontend() {
    if !wasm_tests_enabled() { return; }

    let dir = tempfile::tempdir().unwrap();
    assert_cmd::Command::cargo_bin("my-tool").unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert().success();

    let status = std::process::Command::new("wasm-pack")
        .args(["build", "--target", "web", "--dev"])
        .current_dir(dir.path().join("test-app/frontend"))
        .status()
        .expect("failed to invoke wasm-pack");

    assert!(status.success());
    assert!(dir.path().join("test-app/frontend/pkg").exists());
}
```

#### Acceptance Criteria

- All unit tests pass.
- `run_wasm_pack_with_env` with an empty `PATH` returns `Err`.
- `run_wasm_pack` pointed at an empty directory returns `Err(BuildError::WasmPackFailed)`.
- `run_command_with_timeout` with a long-running command and a short timeout returns `Err(BuildError::Timeout)`.
- When `RUN_WASM_TESTS=1`, the integration test confirms a scaffolded frontend builds successfully.
- `cargo run` in a scaffolded project builds the frontend, starts the server, and exits non-zero if the build fails.

---

### Stage 2 — Dev Server

#### Overview

Add static file serving for `pkg/` contents and a catch-all SPA fallback. Confirm that frontend and backend communicate over the same origin with no CORS configuration.

#### Implementation Plan

- Implement two handlers in `static_assets.rs`:
  - `serve_pkg_file` — extracts the filename from the URL path, applies a **path traversal guard** (see below), resolves against `config.pkg_output_path`, reads the file, and returns it with a MIME type from `mime_guess`.
  - `spa_fallback` — reads `config.index_html_path` and returns it as `text/html; charset=utf-8`.
- **Path traversal guard:** Extract only the final path component using `std::path::Path::new(&filename).file_name()`. If `file_name()` returns `None` (indicating a path with no final component, such as `..`), return HTTP 400. Join only the sanitized filename against `pkg_output_path`. This ensures `/pkg/../../etc/passwd` can never resolve outside the `pkg/` directory.
- Register routes in `main.rs` in this exact order: API routes first, then `/pkg/{filename}`, then `spa_fallback` as `default_service`.

#### Test Plan

**Unit tests** — in `static_assets.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, web};
    use tempfile::tempdir;

    // Note: do not annotate the return type of App::new() chains.
    // Let type inference handle the complex associated types.

    #[actix_web::test]
    async fn serves_js_file_from_pkg_directory() {
        let pkg_dir = tempdir().unwrap();
        std::fs::write(pkg_dir.path().join("app.js"), b"console.log('hi')").unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        let req = test::TestRequest::get().uri("/pkg/app.js").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 200);
    }

    #[actix_web::test]
    async fn serves_wasm_with_application_wasm_content_type() {
        let pkg_dir = tempdir().unwrap();
        std::fs::write(pkg_dir.path().join("app_bg.wasm"), b"\0asm").unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        let req = test::TestRequest::get().uri("/pkg/app_bg.wasm").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/wasm"), "unexpected content-type: {ct}");
    }

    #[actix_web::test]
    async fn returns_404_for_missing_pkg_file() {
        let pkg_dir = tempdir().unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        let req = test::TestRequest::get().uri("/pkg/nonexistent.js").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 404);
    }

    #[actix_web::test]
    async fn path_traversal_attempt_is_rejected() {
        let pkg_dir = tempdir().unwrap();
        let secret = pkg_dir.path().parent().unwrap().join("secret.txt");
        std::fs::write(&secret, b"secret content").unwrap();
        let config = BuildConfig {
            pkg_output_path: pkg_dir.path().to_path_buf(),
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/pkg/{filename}", web::get().to(serve_pkg_file))
        ).await;
        // URL-encoded traversal: /pkg/..%2Fsecret.txt
        let req = test::TestRequest::get()
            .uri("/pkg/..%2Fsecret.txt")
            .to_request();
        let status = test::call_service(&app, req).await.status();
        assert_ne!(status.as_u16(), 200,
            "path traversal must not return 200");
    }

    #[actix_web::test]
    async fn spa_fallback_serves_index_html() {
        let dir = tempdir().unwrap();
        let index = dir.path().join("index.html");
        std::fs::write(&index, b"<html></html>").unwrap();
        let config = BuildConfig {
            index_html_path: index,
            ..BuildConfig::new("/unused")
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .default_service(web::get().to(spa_fallback))
        ).await;
        let req = test::TestRequest::get().uri("/any/unknown/path").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("text/html"), "unexpected content-type: {ct}");
    }

    #[actix_web::test]
    async fn api_route_takes_precedence_over_spa_fallback() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), b"<html></html>").unwrap();
        let config = BuildConfig {
            index_html_path: dir.path().join("index.html"),
            ..BuildConfig::new("/unused")
        };
        async fn hello() -> impl actix_web::Responder {
            actix_web::HttpResponse::Ok()
                .json(serde_json::json!({"message": "hi"}))
        }
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .route("/api/hello", web::get().to(hello))
                .default_service(web::get().to(spa_fallback))
        ).await;
        let req = test::TestRequest::get().uri("/api/hello").to_request();
        let resp = test::call_service(&app, req).await;
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(!ct.contains("text/html"),
            "API route returned index.html instead of JSON");
    }
}
```

#### Acceptance Criteria

- All unit tests pass.
- `GET /pkg/app_bg.wasm` returns 200 with `Content-Type: application/wasm`.
- `GET /pkg/nonexistent.js` returns 404.
- A path traversal request returns 400 or 404, never 200.
- `GET /any/unknown/path` returns `index.html` with status 200.
- `GET /api/hello` returns JSON, not `index.html`.
- The Yew app in the browser communicates with `/api/hello` with no CORS errors.

---

### Stage 3 — File Watch and Rebuild

#### Overview

Add a file watcher that monitors `frontend/src/` and re-invokes `wasm-pack` when `.rs` files change. Builds are serialized: at most one build runs at a time, with one queued if a change arrives during an active build.

#### Implementation Plan

- Implement `start_watcher(watch_path: &Path, debounce_ms: u64, build_tx: mpsc::Sender<()>) -> notify_debouncer_mini::Debouncer<RecommendedWatcher>` in `watcher.rs`. The debouncer callback filters for `.rs` extension changes only and sends `()` on `build_tx`.
  - **The returned `Debouncer` must be stored by the caller for the lifetime of the program.** Dropping it silently stops all watching. Add a prominent doc comment to this effect.
- Implement `run_build_loop<F, Fut>(mut build_rx: mpsc::Receiver<()>, build_fn: F)` where `F: Fn() -> Fut + Send + 'static` and `Fut: Future<Output = Result<(), BuildError>> + Send`. Using a generic `build_fn` makes the loop testable with a mock. The loop logic:
  - Wait for a message on `build_rx`.
  - Start `build_fn()`. While it is running, drain any further messages from `build_rx` and set a `pending: bool` flag.
  - When `build_fn` completes (success or failure), if `pending` is true, start another build immediately and reset the flag.
  - If `build_rx` is closed (sender dropped), exit the loop.
- In `BuildSubsystem::start()`, after the initial build: store the `Debouncer` in a field of `BuildSubsystem`, spawn `run_build_loop` as `tokio::spawn`.
- **Graceful shutdown:** In `main.rs`, add a `tokio::signal::ctrl_c()` handler. On Ctrl-C, drop the `BuildSubsystem` struct (dropping the `Debouncer` and stopping the watcher) and call `server.stop(true)` to drain in-flight requests before exiting. Any `wasm-pack` subprocess currently running will be abandoned — its `Child` handle is held by the build loop task and will be dropped when the task is cancelled.
- **Port binding error:** Catch the `AddrInUse` error from `HttpServer::bind` in `main.rs` and print the human-readable message described in Section 6.

#### Test Plan

**Unit tests** — in `build_coordinator.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn single_trigger_causes_one_build() {
        let (tx, rx) = mpsc::channel(8);
        let count = Arc::new(Mutex::new(0u32));
        let c = count.clone();
        tx.send(()).await.unwrap();
        drop(tx);
        run_build_loop(rx, move || {
            let c = c.clone();
            async move { *c.lock().unwrap() += 1; Ok(()) }
        }).await;
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn rapid_triggers_collapse_to_at_most_two_builds() {
        let (tx, rx) = mpsc::channel(8);
        let count = Arc::new(Mutex::new(0u32));
        let c = count.clone();
        for _ in 0..5 { tx.send(()).await.unwrap(); }
        drop(tx);
        run_build_loop(rx, move || {
            let c = c.clone();
            async move { *c.lock().unwrap() += 1; Ok(()) }
        }).await;
        assert!(*count.lock().unwrap() <= 2,
            "expected at most 2 builds, got {}", *count.lock().unwrap());
    }

    #[tokio::test]
    async fn build_failure_does_not_block_subsequent_builds() {
        let (tx, rx) = mpsc::channel(8);
        let count = Arc::new(Mutex::new(0u32));
        let first = Arc::new(Mutex::new(true));
        let c = count.clone();
        let f = first.clone();
        for _ in 0..2 { tx.send(()).await.unwrap(); }
        drop(tx);
        run_build_loop(rx, move || {
            let c = c.clone();
            let f = f.clone();
            async move {
                *c.lock().unwrap() += 1;
                let mut is_first = f.lock().unwrap();
                if *is_first { *is_first = false; Err(BuildError::WasmPackFailed { exit_code: 1 }) }
                else { Ok(()) }
            }
        }).await;
        assert_eq!(*count.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn closed_channel_exits_loop_cleanly() {
        let (tx, rx) = mpsc::channel::<()>(8);
        drop(tx); // close immediately
        let count = Arc::new(Mutex::new(0u32));
        let c = count.clone();
        run_build_loop(rx, move || {
            let c = c.clone();
            async move { *c.lock().unwrap() += 1; Ok(()) }
        }).await;
        assert_eq!(*count.lock().unwrap(), 0);
    }
}
```

**Integration tests** — in `tests/watcher.rs`:

```rust
use tokio::sync::mpsc;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn rs_file_change_triggers_build_signal() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("lib.rs"), b"// initial").unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let _watcher = start_watcher(dir.path(), 50, tx);

    tokio::time::sleep(Duration::from_millis(200)).await;
    std::fs::write(src.join("lib.rs"), b"// modified").unwrap();

    let got = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(got.is_ok(), "no build signal after .rs file change");
}

#[tokio::test]
async fn non_rs_file_does_not_trigger_build_signal() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let _watcher = start_watcher(dir.path(), 50, tx);

    tokio::time::sleep(Duration::from_millis(200)).await;
    std::fs::write(src.join("notes.txt"), b"not rust").unwrap();

    let got = tokio::time::timeout(Duration::from_millis(600), rx.recv()).await;
    assert!(got.is_err(), "non-.rs change should not trigger a build signal");
}

#[tokio::test]
async fn dropping_watcher_stops_events() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let watcher = start_watcher(dir.path(), 50, tx);
    drop(watcher); // stop watching

    tokio::time::sleep(Duration::from_millis(200)).await;
    std::fs::write(src.join("lib.rs"), b"// change after drop").unwrap();

    let got = tokio::time::timeout(Duration::from_millis(600), rx.recv()).await;
    assert!(got.is_err(), "should receive no signal after watcher is dropped");
}
```

#### Acceptance Criteria

- All unit tests pass.
- Rapid file saves trigger at most two `wasm-pack` invocations.
- A failed build does not prevent subsequent builds.
- Only `.rs` file changes trigger a rebuild.
- Dropping the `Debouncer` stops all watching (confirmed by the drop test).
- Ctrl-C exits cleanly with no zombie processes.
- Actix serves requests normally during a rebuild.

---

### Stage 4 — Live Reload

#### Overview

Add a WebSocket endpoint and a broadcast-based reload mechanism. Inject a reconnecting reload script into `index.html` responses in dev mode. Compile out the endpoint and script in `embed-assets` mode.

#### Implementation Plan

- Define `pub struct DevMode(pub bool)` in `build_subsystem/mod.rs`. Register as `web::Data<DevMode>` in the Actix app.
- In `reload.rs`, define:
  - `pub const RELOAD_SCRIPT: &str` — the complete `<script>...</script>` block from Section 4.3, including the `onclose` reconnect loop.
  - `pub fn inject_reload_script(html: &str) -> String` — inserts `RELOAD_SCRIPT` before the last `</body>` occurrence. If `</body>` is absent, appends at the end.
  - `pub async fn ws_handler(...)` — upgrades via `actix_ws::handle`, spawns a task subscribed to the broadcast channel, sends `"reload"` text frames on each broadcast.
- Update `run_build_loop` to accept a `broadcast::Sender<()>`. After a successful build: `let _ = sender.send(())`. The `let _` explicitly silences the `Err` that occurs when there are no active subscribers — this is expected and must not be logged as an error.
- In `spa_fallback`, check `DevMode` from app data; if `DevMode(true)`, call `inject_reload_script` on the file contents before returning.
- In `main.rs`, gate the `/ws/reload` route registration behind `#[cfg(not(feature = "embed-assets"))]`.

#### Test Plan

**Unit tests** — in `reload.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_is_injected_before_body_close_tag() {
        let html = "<html><body><p>Hello</p></body></html>";
        let result = inject_reload_script(html);
        let script_pos = result.find(RELOAD_SCRIPT).expect("script not found");
        let body_pos = result.find("</body>").expect("</body> not found");
        assert!(script_pos < body_pos);
    }

    #[test]
    fn handles_html_without_body_tag_gracefully() {
        let html = "<html><p>No body tag</p></html>";
        let result = inject_reload_script(html);
        assert!(result.contains(RELOAD_SCRIPT));
    }

    #[test]
    fn reload_script_references_ws_reload_path() {
        assert!(RELOAD_SCRIPT.contains("/ws/reload"));
    }

    #[test]
    fn reload_script_contains_onclose_reconnect() {
        assert!(RELOAD_SCRIPT.contains("onclose"),
            "reload script must include reconnect logic — see Section 4.3");
    }

    #[test]
    fn broadcast_send_with_no_receivers_does_not_panic() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<()>(16);
        drop(_rx);
        let result = tx.send(());
        let _ = result; // Err is expected and acceptable
    }
}
```

**Integration tests** — in `tests/reload.rs`:

```rust
use actix_web::{test, App, web};
use tokio::sync::broadcast;
use tempfile::tempdir;

#[actix_web::test]
async fn ws_endpoint_returns_101_switching_protocols() {
    let (tx, _rx) = broadcast::channel::<()>(16);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(tx))
            .route("/ws/reload", web::get().to(ws_handler))
    ).await;
    let req = test::TestRequest::get()
        .uri("/ws/reload")
        .insert_header(("upgrade", "websocket"))
        .insert_header(("connection", "upgrade"))
        .insert_header(("sec-websocket-version", "13"))
        .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 101);
}

#[actix_web::test]
async fn index_html_contains_reload_script_when_dev_mode_true() {
    let dir = tempdir().unwrap();
    let index = dir.path().join("index.html");
    std::fs::write(&index, b"<html><body></body></html>").unwrap();
    let config = BuildConfig { index_html_path: index, ..BuildConfig::new("/unused") };
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(DevMode(true)))
            .default_service(web::get().to(spa_fallback))
    ).await;
    let body = test::read_body(
        test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await
    ).await;
    assert!(std::str::from_utf8(&body).unwrap().contains("/ws/reload"));
}

#[actix_web::test]
async fn index_html_omits_reload_script_when_dev_mode_false() {
    let dir = tempdir().unwrap();
    let index = dir.path().join("index.html");
    std::fs::write(&index, b"<html><body></body></html>").unwrap();
    let config = BuildConfig { index_html_path: index, ..BuildConfig::new("/unused") };
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(DevMode(false)))
            .default_service(web::get().to(spa_fallback))
    ).await;
    let body = test::read_body(
        test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await
    ).await;
    assert!(!std::str::from_utf8(&body).unwrap().contains("/ws/reload"));
}
```

#### Acceptance Criteria

- All unit tests pass.
- `GET /ws/reload` returns HTTP 101.
- `index.html` in dev mode contains the reload script with `onclose` reconnect.
- `index.html` in release mode (`DevMode(false)`) does not contain the reload script.
- Broadcast with no subscribers does not panic or log an error.
- After a successful rebuild, browser tabs reload automatically.
- After a server restart, the browser reconnects and reloads on the next build.

---

### Stage 5 — Release Embedding

#### Overview

Add the `embed-assets` feature. The `build.rs` guard uses the `CARGO_FEATURE_EMBED_ASSETS` environment variable. `include_dir` embeds `frontend/pkg/` and `include_bytes!` embeds `frontend/index.html`. Dev-mode code is compiled out.

#### Implementation Plan

- In `static_assets.rs`, add:

```rust
#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

#[cfg(feature = "embed-assets")]
static EMBEDDED_INDEX: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/index.html"));
```

- Add `#[cfg(feature = "embed-assets")]` and `#[cfg(not(feature = "embed-assets"))]` branches to both handlers.
- In `backend/build.rs`:

```rust
fn main() {
    // Use the environment variable Cargo sets for features — NOT cfg!()
    // cfg!() does not reflect Cargo features in build.rs.
    if std::env::var("CARGO_FEATURE_EMBED_ASSETS").is_ok() {
        let pkg = std::path::Path::new("../frontend/pkg");
        if !pkg.exists() {
            panic!(
                "\n\nERROR: frontend/pkg/ does not exist.\n\
                 Run before building with --features embed-assets:\n\n    \
                 wasm-pack build frontend/ --target web --release\n\n"
            );
        }
        if pkg.read_dir().unwrap().next().is_none() {
            panic!(
                "\n\nERROR: frontend/pkg/ is empty.\n\
                 Run: wasm-pack build frontend/ --target web --release\n\n"
            );
        }
    }
    println!("cargo:rerun-if-changed=../frontend/pkg");
    println!("cargo:rerun-if-changed=../frontend/index.html");
}
```

- In `main.rs`, gate `BuildSubsystem::start()` and `/ws/reload` behind `#[cfg(not(feature = "embed-assets"))]`.

#### Test Plan

**Unit tests** — in `static_assets.rs`:

```rust
#[cfg(all(test, feature = "embed-assets"))]
mod embed_tests {
    use super::*;

    #[test]
    fn embedded_pkg_is_non_empty() {
        assert!(EMBEDDED_PKG.entries().count() > 0);
    }

    #[test]
    fn embedded_pkg_contains_wasm_file() {
        let has_wasm = EMBEDDED_PKG.entries().any(|e| {
            e.path().extension().map(|x| x == "wasm").unwrap_or(false)
        });
        assert!(has_wasm, "no .wasm file in embedded pkg/");
    }

    #[test]
    fn embedded_index_is_non_empty() {
        assert!(!EMBEDDED_INDEX.is_empty());
    }

    #[test]
    fn embedded_index_does_not_contain_reload_script() {
        let html = std::str::from_utf8(EMBEDDED_INDEX).unwrap();
        assert!(!html.contains("/ws/reload"));
    }
}
```

#### Acceptance Criteria

- `cargo build --features embed-assets` without a `frontend/pkg/` directory fails with the panic message containing "frontend/pkg/ does not exist".
- `backend/build.rs` contains `CARGO_FEATURE_EMBED_ASSETS` and does **not** contain `cfg!(feature`.
- After `wasm-pack build frontend/ --target web --release`, `cargo build --release --features embed-assets` succeeds.
- The release binary serves `app_bg.wasm` with `Content-Type: application/wasm` after `frontend/pkg/` is deleted from disk.
- `GET /ws/reload` on the release binary returns 404.
- `index.html` served by the release binary does not contain the reload script.

---

### Stage 6 — Shared Types Integration

#### Overview

Establish the shared types pattern as a working, tested foundation. Confirm the compile-time contract, the forward-compatibility stance, and WASM target compatibility.

**Serde forward-compatibility rule:** No type in `shared` may use `#[serde(deny_unknown_fields)]`. See Section 5.2. This is enforced by the `unknown_fields_are_ignored_not_rejected` test below and the Stage 0 scaffold test. Future additions to `shared` must follow this rule.

**Yew version:** All Yew frontend code targets Yew **0.21**. The `use_effect_with((), ...)` hook signature is specific to 0.21. Do not silently upgrade to a newer version.

#### Implementation Plan

- Ensure `shared/src/lib.rs` contains both `HelloResponse` and `StatusResponse` as in Section 5.3, with no `deny_unknown_fields`.
- Add `/api/status` handler to `backend/src/api/mod.rs`, returning `StatusResponse { version: env!("CARGO_PKG_VERSION").into(), uptime_seconds: 0 }`.
- Register `/api/status` in `main.rs`.
- Update `frontend/src/lib.rs` to fetch `/api/status` on mount using `use_effect_with((), ...)` and display the version string.
- Add the compile-time trait assertion block to `shared/src/lib.rs`.

#### Test Plan

**Unit tests** — in `shared/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_response_round_trips_through_json() {
        let r = HelloResponse { message: "hi".into() };
        let json = serde_json::to_string(&r).unwrap();
        let r2: HelloResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.message, "hi");
    }

    #[test]
    fn status_response_round_trips_through_json() {
        let r = StatusResponse { version: "0.1.0".into(), uptime_seconds: 42 };
        let json = serde_json::to_string(&r).unwrap();
        let r2: StatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.version, "0.1.0");
        assert_eq!(r2.uptime_seconds, 42);
    }

    #[test]
    fn unknown_fields_are_ignored_not_rejected() {
        // Verifies deny_unknown_fields is absent — see Section 5.2
        let json = r#"{"message":"hi","future_field":true}"#;
        let r: HelloResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.message, "hi");
    }
}

// Compile-time assertions: field renames or trait removals become
// compile errors rather than runtime failures.
const _: fn() = || {
    fn assert_serialize<T: serde::Serialize>() {}
    fn assert_deserialize<T: for<'de> serde::Deserialize<'de>>() {}
    assert_serialize::<HelloResponse>();
    assert_deserialize::<HelloResponse>();
    assert_serialize::<StatusResponse>();
    assert_deserialize::<StatusResponse>();
};
```

**Integration tests** — in `tests/api.rs`:

```rust
use actix_web::{test, App, web};
use shared::{HelloResponse, StatusResponse};

#[actix_web::test]
async fn hello_returns_non_empty_message() {
    let app = test::init_service(
        App::new().route("/api/hello", web::get().to(hello_handler))
    ).await;
    let body: HelloResponse = test::read_body_json(
        test::call_service(
            &app,
            test::TestRequest::get().uri("/api/hello").to_request()
        ).await
    ).await;
    assert!(!body.message.is_empty());
}

#[actix_web::test]
async fn status_returns_non_empty_version() {
    let app = test::init_service(
        App::new().route("/api/status", web::get().to(status_handler))
    ).await;
    let body: StatusResponse = test::read_body_json(
        test::call_service(
            &app,
            test::TestRequest::get().uri("/api/status").to_request()
        ).await
    ).await;
    assert!(!body.version.is_empty());
}

#[actix_web::test]
async fn api_responses_have_json_content_type() {
    let app = test::init_service(
        App::new().route("/api/hello", web::get().to(hello_handler))
    ).await;
    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/hello").to_request()
    ).await;
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("application/json"), "unexpected content-type: {ct}");
}
```

#### Acceptance Criteria

- All unit tests in `shared` pass on the host target.
- `cargo check --target wasm32-unknown-unknown -p shared` succeeds.
- `GET /api/hello` returns a JSON body that deserialises into `HelloResponse`.
- `GET /api/status` returns a JSON body that deserialises into `StatusResponse`.
- A JSON string with an unknown field deserialises into `HelloResponse` without error.
- Renaming a field in `HelloResponse` without updating the backend handler is a compile error.
- All API responses carry `Content-Type: application/json`.

---

## 9. Known Risks and Open Questions

### 9.1 Risks

- **wasm-pack version coupling:** The build subsystem shells out to whatever `wasm-pack` is on `PATH`. A breaking change to the `wasm-pack` CLI would break silently without the version check. Mitigation: the environment check at startup verifies the version is >= 0.13.0 and prints an actionable error if not.
- **Concurrent builds:** Rapid file saves could trigger overlapping `wasm-pack` invocations without the coordinator. Mitigation: the build coordinator's pending-flag pattern in Stage 3 serializes all builds; this is covered by unit tests.
- **Windows path handling:** `PathBuf` and shell-out behaviour differ on Windows. This tool targets EndeavourOS Linux and Windows support is not a goal, but path operations should avoid hardcoded separators.
- **Watcher lifetime:** Dropping the `Debouncer` silently stops file watching. Mitigation: stored in a long-lived struct, and the drop behaviour is verified by a dedicated test.

### 9.2 Resolved Design Decisions

- **`index.html` location:** Lives in `frontend/`. The backend reads it via `BuildConfig::index_html_path`, which defaults to `frontend_crate_path + "/index.html"`.
- **WebSocket reconnect:** The injected reload script includes an `onclose` reconnect loop (1-second retry), resolving the concern about connection loss on server restart.
- **wasm-opt integration:** `wasm-pack` invokes `wasm-opt` automatically in release mode if the binary is available. The build subsystem does not manage `wasm-opt` separately.
- **`#[actix_web::main]` vs `#[tokio::main]`:** Generated `main.rs` uses `#[actix_web::main]` to avoid Actix runtime conflicts.
- **`cfg!()` in `build.rs`:** Does not work for Cargo feature detection. Use `std::env::var("CARGO_FEATURE_EMBED_ASSETS")` instead.
- **`frontend-builder` crate:** There is no separate `frontend-builder` crate. The build subsystem code is generated inline into the backend crate by `init` and owned by the project.
- **`deny_unknown_fields`:** Prohibited on all `shared` types. Forward-compatibility requires that unknown fields are silently ignored.

### 9.3 Open Questions

- **Backend hot reload:** Changes to the backend source still require a manual restart. Integrating `cargo-watch`-style backend restarting would complete the single-terminal story but adds significant complexity. Deferred.
- **Build error overlay:** A browser-visible overlay showing compiler errors would improve the dev experience. Worth considering after the core pipeline is stable.

---

## 10. Instrumentation and Error Handling

Error types, `tracing` span placement, log level policy, and the panic policy are specified in the companion document `instrumentation_and_error_handling.md`. All implementation stages are expected to follow that strategy.

Net-new `Cargo.toml` additions required beyond the dependencies listed in Section 7:

- `thiserror = "1"` — in both `my-tool` and the generated backend template
- `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` — in both, plus workspace-level `[dev-dependencies]`

---

## 11. Future Ideas

- **CSS watching:** Detect changes to static CSS files and trigger a browser reload without a full WASM rebuild.
- **Build error overlay:** Inject a full-screen error overlay into the browser when `wasm-pack` fails, showing the compiler output inline.
- **Backend auto-restart:** Watch the backend crate and restart the server on changes, eliminating the last reason to touch the terminal during development.
- **Incremental builds:** Investigate reducing `wasm-pack` rebuild times by warming the `cargo` incremental compilation cache between runs.

---

_Personal design document — Jeff · Not intended for publication_
