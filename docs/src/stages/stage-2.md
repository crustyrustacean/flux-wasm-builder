# Stage 2: File Watching

Stage 2 implements filesystem watching to detect frontend source changes.

## Overview

**Key Behavior:**
- Watches `frontend/src/**/*.rs` recursively
- Debounces events with 300ms quiet period
- Only `.rs` files trigger rebuild signals
- Uses `notify-debouncer-mini` for cross-platform watching

## Architecture

```
start_watcher(watch_path, debounce_ms, build_tx)
  └─► new_debouncer(Duration::from_millis(debounce_ms), callback)
        └─► callback(DebounceEventResult)
              ├─► Filter for .rs files
              └─► build_tx.blocking_send(())
```

## Implementation Components

### FileWatcher Type

```rust
pub type FileWatcher = notify_debouncer_mini::Debouncer<
    notify_debouncer_mini::notify::RecommendedWatcher
>;
```

### start_watcher Function

```rust
pub fn start_watcher(
    watch_path: &Path,
    debounce_ms: u64,
    build_tx: Sender<()>,
) -> Result<FileWatcher, notify::Error>
```

### Event Filtering

```rust
let rs_change = events.iter().any(|e| {
    e.path.extension()
        .map(|ext| ext == "rs")
        .unwrap_or(false)
});

if rs_change {
    build_tx.blocking_send(())?;
}
```

## Critical: Watcher Lifetime

The `Debouncer` must be stored for the process lifetime:

```rust
// CORRECT: Store in a long-lived struct
struct ServerState {
    watcher: FileWatcher,
}

// WRONG: Local variable dropped at end of scope
fn start() {
    let watcher = start_watcher(...);  // Dropped here!
}
```

Dropping the debouncer silently stops all file watching.

## Channel Communication

The watcher uses `tokio::sync::mpsc` for signaling:

- `blocking_send()` bridges sync callback to async channel
- Callback runs on notify's background thread
- Receiver is in the async build coordinator

## Debouncing

Editors use atomic save strategies that create multiple events:

1. Write to temporary file
2. Rename temp to target file

The debounce window (default 300ms) absorbs these into a single trigger.

## Test Coverage

- `.rs` file change triggers build signal
- Non-`.rs` file does not trigger signal
- Dropping watcher stops events
- Watcher returns error for nonexistent path
- Rapid changes are debounced

## Platform Support

Uses `notify::RecommendedWatcher`:

| Platform | Backend |
|----------|---------|
| Linux | inotify |
| macOS | FSEvents |
| Windows | ReadDirectoryChangesW |

All platforms are supported transparently.
