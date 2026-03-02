# Error Types

flux-wasm-builder uses typed errors via `thiserror` for clear, actionable error messages.

## BuildError

Errors from the build subsystem:

```rust
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// wasm-pack exited with a non-zero status.
    #[error("wasm-pack failed (exit code: {exit_code:?})")]
    WasmPackFailed { 
        exit_code: Option<i32> 
    },

    /// Build timed out and the child process was killed.
    #[error("wasm-pack timed out after {secs}s")]
    Timeout { 
        secs: u64 
    },

    /// The wasm-pack binary could not be found or spawned.
    #[error("failed to spawn wasm-pack: {source}")]
    SpawnFailed { 
        source: std::io::Error 
    },

    /// Waiting on the child process failed (OS error).
    #[error("failed to wait on wasm-pack process: {source}")]
    WaitFailed { 
        source: std::io::Error 
    },

    /// Killing the timed-out process failed.
    #[error("failed to kill timed-out wasm-pack process: {source}")]
    KillFailed { 
        source: std::io::Error 
    },
}
```

### Handling BuildError

```rust
match run_wasm_pack(&config).await {
    Ok(()) => println!("Build succeeded"),
    Err(BuildError::WasmPackFailed { exit_code }) => {
        eprintln!("Compilation failed (exit code: {:?})", exit_code);
    }
    Err(BuildError::Timeout { secs }) => {
        eprintln!("Build timed out after {}s", secs);
    }
    Err(BuildError::SpawnFailed { source }) => {
        eprintln!("Could not run wasm-pack: {}", source);
    }
    Err(e) => eprintln!("Build error: {}", e),
}
```

## InitError

Errors from project scaffolding:

```rust
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// Target directory already exists.
    #[error("directory '{0}' already exists")]
    AlreadyExists(std::path::PathBuf),

    /// Environment check failed.
    #[error("environment check failed: {0}")]
    EnvCheck(#[from] EnvCheckError),

    /// Failed to create directory.
    #[error("failed to create directory '{path}': {source}")]
    CreateDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    /// Failed to write file.
    #[error("failed to write '{path}': {source}")]
    WriteFile {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}
```

### Handling InitError

```rust
match scaffold(&path, "my-app") {
    Ok(()) => println!("Project created"),
    Err(InitError::AlreadyExists(path)) => {
        eprintln!("Error: {} already exists", path.display());
    }
    Err(InitError::EnvCheck(e)) => {
        eprintln!("Prerequisite error: {}", e);
    }
    Err(InitError::CreateDir { path, source }) => {
        eprintln!("Could not create {}: {}", path.display(), source);
    }
    Err(e) => eprintln!("Init error: {}", e),
}
```

## EnvCheckError

Errors from environment validation:

```rust
#[derive(Debug, thiserror::Error)]
pub enum EnvCheckError {
    /// wasm-pack not found on PATH.
    #[error("wasm-pack not found on PATH — install from https://rustwasm.github.io/wasm-pack/")]
    WasmPackNotFound,

    /// wasm-pack version is below minimum required.
    #[error("wasm-pack version {found} is below minimum required {required}")]
    WasmPackVersionTooOld { 
        found: String, 
        required: String 
    },

    /// wasm32-unknown-unknown target not installed.
    #[error("wasm32-unknown-unknown target not installed — run: rustup target add wasm32-unknown-unknown")]
    Wasm32TargetMissing,

    /// Failed to invoke external command.
    #[error("failed to invoke '{command}': {source}")]
    SpawnFailed {
        command: String,
        source: std::io::Error,
    },

    /// Failed to parse wasm-pack version output.
    #[error("failed to parse wasm-pack version output: {0}")]
    VersionParseFailed(String),
}
```

### Handling EnvCheckError

```rust
match wasm_pack_version_ok() {
    Ok(()) => println!("wasm-pack version OK"),
    Err(EnvCheckError::WasmPackNotFound) => {
        eprintln!("Install wasm-pack from https://rustwasm.github.io/wasm-pack/");
    }
    Err(EnvCheckError::WasmPackVersionTooOld { found, required }) => {
        eprintln!("Update wasm-pack: found {}, need {}", found, required);
    }
    Err(EnvCheckError::Wasm32TargetMissing) => {
        eprintln!("Run: rustup target add wasm32-unknown-unknown");
    }
    Err(e) => eprintln!("Environment error: {}", e),
}
```

## Error Propagation

All error types implement `std::error::Error` and can be propagated with `?`:

```rust
fn create_project(path: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Environment checks
    wasm_pack_version_ok()?;
    wasm32_target_installed()?;
    
    // Scaffolding
    scaffold(path, name)?;
    
    Ok(())
}
```

## Error Messages

Error messages are designed to be actionable:

| Error | Message |
|-------|---------|
| `WasmPackNotFound` | "wasm-pack not found on PATH — install from https://rustwasm.github.io/wasm-pack/" |
| `Wasm32TargetMissing` | "wasm32-unknown-unknown target not installed — run: rustup target add wasm32-unknown-unknown" |
| `AlreadyExists` | "directory 'my-app' already exists" |

Users can copy-paste commands directly from error messages.

## Logging Errors

Use `tracing` for structured error logging:

```rust
if let Err(e) = run_wasm_pack(&config).await {
    tracing::error!(
        error = %e,
        frontend = %config.frontend_crate_path.display(),
        "build failed"
    );
}
```

## Testing Errors

```rust
#[test]
fn timeout_error_displays_seconds() {
    let err = BuildError::Timeout { secs: 60 };
    let msg = err.to_string();
    assert!(msg.contains("60"));
    assert!(msg.contains("timed out"));
}

#[test]
fn env_check_error_has_install_url() {
    let err = EnvCheckError::WasmPackNotFound;
    let msg = err.to_string();
    assert!(msg.contains("rustwasm.github.io/wasm-pack"));
}
```
