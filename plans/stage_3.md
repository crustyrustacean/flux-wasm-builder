# Stage 3 Implementation Plan: File Watch and Rebuild

## Overview

Stage 3 adds a file watcher that monitors `frontend/src/` and re-invokes `wasm-pack` when `.rs` files change. Builds are serialized: at most one build runs at a time, with at most one additional build queued if changes arrive during an active build.

**Reference:** Section 8, Stage 3 of `flux_wasm_builder_spec.md` (lines 903-1066)
**Instrumentation:** Sections 4.4, 4.5 of `flux_wasm_builder_instrumentation_and_error_handling.md`

---

## Current Codebase Status

### Completed Stages

| Stage | Description | Status |
|-------|-------------|--------|
| Stage 0 | Project Scaffolding (`init`) | ✅ Complete |
| Stage 1 | Static Build Pipeline | ✅ Complete |
| Stage 2 | Dev Server | ✅ Complete |
| Stage 3 | File Watch and Rebuild | 🔲 This Plan |

### Existing Files

```
src/
├── lib.rs                    # Module declarations and re-exports
├── env_check.rs              # Environment verification functions
├── bin/
│   └── main.rs               # CLI entry point (clap)
├── build/
│   ├── mod.rs                # Build module exports
│   ├── build.rs              # wasm-pack invocation, BuildConfig, BuildError
│   └── static_assets.rs      # serve_pkg_file, spa_fallback handlers
└── init/
    ├── mod.rs                # scaffold function
    └── templates.rs          # Generated project templates
```

### Current Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["process", "time", "rt", "rt-multi-thread", "macros", "fs"] }
thiserror = "2.0.18"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
actix-web = "4"
mime_guess = "2"
serde_json = "1"
```

---

## Implementation Components

### 1. Add Dependencies

**File:** `Cargo.toml`

Add `notify-debouncer-mini` for filesystem watching:

```toml
[dependencies]
# ... existing dependencies ...
notify-debouncer-mini = "0.4"
```

**Rationale:** Per spec Section 4.2, `notify-debouncer-mini` provides built-in debouncing with a configurable quiet period, avoiding the need to implement timer logic manually.

---

### 2. Create `watcher.rs` Module

**File:** `src/build/watcher.rs`

#### Function Signature

```rust
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;
use notify_debouncer_mini::{Debouncer, RecommendedWatcher, new_debouncer, DebounceEventResult};
use notify::RecursiveMode;

/// Start the file watcher.
///
/// # Important — Lifetime
/// The returned `Debouncer` **must be stored** for the lifetime of the process.
/// Dropping it silently stops all file watching with no error or warning.
/// Store it in a long-lived struct field, never in a local variable.
pub fn start_watcher(
    watch_path: &Path,
    debounce_ms: u64,
    build_tx: Sender<()>,
) -> Result<Debouncer<RecommendedWatcher>, notify::Error> {
    // Implementation
}
```

#### Key Implementation Details

1. **Debounce Configuration:** Use `new_debouncer` with `Duration::from_millis(debounce_ms)`
2. **Event Filtering:** Only `.rs` file changes trigger `build_tx.send(())`
3. **Error Handling in Callback:**
   - `DebounceEventResult::Ok(events)` → filter for `.rs` extensions
   - `DebounceEventResult::Err(errors)` → log at `warn!` level, continue watching
4. **Channel Send Failure:** If `blocking_send(())` fails, the receiver has been dropped (server shutting down). Log at `debug!` and continue.

#### Tracing Instrumentation

```rust
let span = tracing::info_span!(
    "file_watcher",
    path = %watch_path.display(),
    debounce_ms
);
```

- `info!` on watcher start
- `debug!` on each `.rs` change detected
- `trace!` for ignored non-`.rs` events
- `warn!` for watcher errors

---

### 3. Create `build_coordinator.rs` Module

**File:** `src/build/build_coordinator.rs`

#### Function Signature

```rust
use tokio::sync::mpsc::Receiver;
use std::future::Future;
use super::build::BuildError;

/// Run the build loop, serializing rebuild requests.
///
/// The loop waits for signals on `build_rx`, executes `build_fn`,
/// and coalesces rapid triggers into at most two builds (one running, one pending).
pub async fn run_build_loop<F, Fut>(
    mut build_rx: Receiver<()>,
    build_fn: F,
) 
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), BuildError>> + Send,
{
    // Implementation
}
```

#### Loop Logic (Per Spec)

```
┌─────────────────────────────────────────────────────────────┐
│                    Build Loop State Machine                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐                                           │
│  │   WAITING    │◄─────────────────────────────────┐        │
│  │  for signal  │                                   │        │
│  └──────┬───────┘                                   │        │
│         │ recv()                                    │        │
│         ▼                                           │        │
│  ┌──────────────┐                                   │        │
│  │   DRAIN      │  while try_recv().is_ok()        │        │
│  │  coalesce    │  → set pending = true            │        │
│  └──────┬───────┘                                   │        │
│         │                                           │        │
│         ▼                                           │        │
│  ┌──────────────┐                                   │        │
│  │   BUILDING   │  execute build_fn()              │        │
│  │  one build   │  (success or failure logged)     │        │
│  └──────┬───────┘                                   │        │
│         │                                           │        │
│         ▼                                           │        │
│  ┌──────────────┐                                   │        │
│  │   CHECK      │  if pending:                      │        │
│  │   pending?   │    pending = false                │        │
│  └──────┬───────┘    loop immediately               │        │
│         │                                           │        │
│         │ no pending                                │        │
│         └───────────────────────────────────────────┘        │
│                                                              │
│  Exit condition: build_rx closed (sender dropped)           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Key Implementation Details

1. **Coalescing Logic:**
   - After `recv().await` returns, drain all queued messages with `try_recv()`
   - Set `pending: bool` flag if any additional messages were drained
   - After build completes, if `pending` is true, immediately start another build

2. **Error Handling:**
   - Build failure is logged at `warn!` level (not `error!`)
   - Failed builds do NOT exit the loop — continue watching
   - Previous successful assets remain served

3. **Channel Close:**
   - `recv()` returning `None` indicates sender dropped
   - Exit loop cleanly with `debug!` log

#### Tracing Instrumentation

```rust
let span = tracing::info_span!("rebuild_cycle");
```

- `info!` on successful rebuild
- `warn!` on failed rebuild (with error message)
- `debug!` on coalesced triggers
- `debug!` on channel close (loop exit)

---

### 4. Update `build/mod.rs`

**File:** `src/build/mod.rs`

Add module declarations and exports:

```rust
mod build;
mod build_coordinator;
mod static_assets;
mod watcher;

pub use build::{BuildConfig, BuildError, run_wasm_pack, run_wasm_pack_with_env, run_command_with_timeout};
pub use build_coordinator::run_build_loop;
pub use static_assets::{serve_pkg_file, spa_fallback};
pub use watcher::start_watcher;
```

---

### 5. Update `lib.rs`

**File:** `src/lib.rs`

Add re-exports for new modules:

```rust
pub mod build;
pub mod env_check;
pub mod init;

pub use build::{
    BuildConfig, BuildError, 
    run_wasm_pack, run_wasm_pack_with_env, run_command_with_timeout,
    run_build_loop, start_watcher,
};
pub use env_check::EnvCheckError;
pub use init::{scaffold, InitError};
```

---

### 6. Unit Tests

#### 6.1 `watcher.rs` Tests

**File:** `src/build/watcher.rs` (inline `#[cfg(test)]` module)

| Test | Description |
|------|-------------|
| `rs_file_change_triggers_build_signal` | Writing to a `.rs` file causes a signal on the channel |
| `non_rs_file_does_not_trigger_build_signal` | Writing to a `.txt` file does NOT trigger a signal |
| `dropping_watcher_stops_events` | After dropping the `Debouncer`, no more signals are sent |

**Test Infrastructure Notes:**
- Use `tempfile::tempdir()` for isolated test directories
- Use `tokio::time::sleep` for debounce settling (200ms initial + event timing)
- Use `tokio::time::timeout` to avoid hanging tests

#### 6.2 `build_coordinator.rs` Tests

**File:** `src/build/build_coordinator.rs` (inline `#[cfg(test)]` module)

| Test | Description |
|------|-------------|
| `single_trigger_causes_one_build` | One send → one build execution |
| `rapid_triggers_collapse_to_at_most_two_builds` | 5 rapid sends → at most 2 builds |
| `build_failure_does_not_block_subsequent_builds` | First build fails, second succeeds, both execute |
| `closed_channel_exits_loop_cleanly` | Dropping sender exits loop without builds |

**Test Pattern:**

```rust
#[tokio::test]
async fn rapid_triggers_collapse_to_at_most_two_builds() {
    let (tx, rx) = mpsc::channel(8);
    let count = Arc::new(Mutex::new(0u32));
    let c = count.clone();
    
    for _ in 0..5 { tx.send(()).await.unwrap(); }
    drop(tx);  // Close channel to allow loop to exit
    
    run_build_loop(rx, move || {
        let c = c.clone();
        async move { *c.lock().unwrap() += 1; Ok(()) }
    }).await;
    
    assert!(*count.lock().unwrap() <= 2);
}
```

---

### 7. Integration Tests

**File:** `tests/watcher.rs`

These tests exercise the full watcher + coordinator integration:

```rust
use flux_wasm_builder::{start_watcher, run_build_loop};
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
    let _watcher = start_watcher(dir.path(), 50, tx).expect("watcher should start");

    tokio::time::sleep(Duration::from_millis(200)).await;
    std::fs::write(src.join("lib.rs"), b"// modified").unwrap();

    let got = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(got.is_ok(), "no build signal after .rs file change");
}
```

---

### 8. Generated Project Template Updates

The generated backend project needs to include the new modules. Update:

**File:** `src/init/templates.rs`

#### 8.1 `build_subsystem/mod.rs` Template

Add module declarations:

```rust
pub mod build;
pub mod build_coordinator;
pub mod static_assets;
pub mod watcher;

pub use build::{BuildConfig, BuildError, run_wasm_pack, run_wasm_pack_with_env};
pub use build_coordinator::run_build_loop;
pub use static_assets::{serve_pkg_file, spa_fallback};
pub use watcher::start_watcher;
```

#### 8.2 `build_subsystem/watcher.rs` Template

Copy the implementation from `src/build/watcher.rs` with appropriate imports.

#### 8.3 `build_subsystem/build_coordinator.rs` Template

Copy the implementation from `src/build/build_coordinator.rs`.

#### 8.4 `backend/Cargo.toml` Template

Add dependency:

```toml
notify-debouncer-mini = "0.4"
```

---

## Acceptance Criteria

Per spec Section 8, Stage 3:

| Criterion | Verification |
|-----------|--------------|
| All unit tests pass | `cargo test --lib` |
| Rapid file saves trigger at most two `wasm-pack` invocations | `rapid_triggers_collapse_to_at_most_two_builds` test |
| A failed build does not prevent subsequent builds | `build_failure_does_not_block_subsequent_builds` test |
| Only `.rs` file changes trigger a rebuild | `non_rs_file_does_not_trigger_build_signal` test |
| Dropping the `Debouncer` stops all watching | `dropping_watcher_stops_events` test |
| Ctrl-C exits cleanly with no zombie processes | Manual verification |
| Actix serves requests normally during a rebuild | Manual verification |

---

## Implementation Order

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Implementation Sequence                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. DEPENDENCIES                                                         │
│     └── Add notify-debouncer-mini to Cargo.toml                         │
│                                                                          │
│  2. WATCHER MODULE (TDD)                                                 │
│     ├── Write tests in src/build/watcher.rs                             │
│     ├── Implement start_watcher()                                        │
│     └── Verify tests pass                                                │
│                                                                          │
│  3. BUILD COORDINATOR MODULE (TDD)                                       │
│     ├── Write tests in src/build/build_coordinator.rs                   │
│     ├── Implement run_build_loop()                                       │
│     └── Verify tests pass                                                │
│                                                                          │
│  4. MODULE EXPORTS                                                       │
│     ├── Update src/build/mod.rs                                          │
│     └── Update src/lib.rs                                                │
│                                                                          │
│  5. INTEGRATION TESTS                                                    │
│     ├── Create tests/watcher.rs                                          │
│     └── Verify all integration tests pass                                │
│                                                                          │
│  6. TEMPLATE UPDATES                                                     │
│     ├── Update build_subsystem/mod.rs template                           │
│     ├── Add watcher.rs template                                          │
│     ├── Add build_coordinator.rs template                                │
│     └── Update backend/Cargo.toml template                               │
│                                                                          │
│  7. VERIFICATION                                                         │
│     ├── Run full test suite: cargo test                                  │
│     ├── Manual end-to-end test with generated project                    │
│     └── Verify Ctrl-C behavior                                           │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `src/build/watcher.rs` | File watcher implementation |
| `src/build/build_coordinator.rs` | Build loop coordinator |
| `tests/watcher.rs` | Integration tests |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `notify-debouncer-mini` dependency |
| `src/build/mod.rs` | Add module declarations and exports |
| `src/lib.rs` | Add re-exports |
| `src/init/templates.rs` | Add new module templates for generated projects |

---

## Notes on Critical Design Decisions

### 1. Debouncer Lifetime

> **The returned `Debouncer` must be stored by the caller for the lifetime of the program.**

This is the most critical design constraint. Dropping the `Debouncer` silently stops all watching with no error. The doc comment must be prominent, and the generated project's `main.rs` must store it in a long-lived struct.

### 2. Build Failure Handling

Build failures are logged at `warn!` level, not `error!`. The server continues running and serving the last successful build. This matches developer expectations during active editing.

### 3. Channel Send Failure

When `blocking_send(())` fails in the watcher callback, it means the receiver (build coordinator) has been dropped. This is expected during shutdown and should be logged at `debug!` level, not treated as an error.

### 4. Coalescing Strategy

The "at most two builds" guarantee comes from:
- One build currently running
- One `pending` flag set if changes arrive during the build
- After the build, if `pending` is true, one more build runs immediately

This prevents build storms while ensuring the final state is always built.

---

## Out of Scope for Stage 3

The following are deferred to Stage 4 (Live Reload):

- WebSocket endpoint (`/ws/reload`)
- Reload script injection into `index.html`
- `broadcast::Sender` for signaling browser reload
- `DevMode` flag and `#[cfg]` gating

---

## References

- `flux_wasm_builder_spec.md` Section 8, Stage 3 (lines 903-1066)
- `flux_wasm_builder_instrumentation_and_error_handling.md` Sections 4.4, 4.5
- `notify-debouncer-mini` documentation: https://docs.rs/notify-debouncer-mini/
