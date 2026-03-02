# File Watching

The file watcher monitors frontend source files and triggers rebuilds when changes are detected.

## Module

| File | Purpose |
|------|---------|
| [`watcher.rs`](https://github.com/crustyrustacean/flux-wasm-builder/blob/main/src/build/watcher.rs) | Filesystem event watching |

## Implementation

### notify-debouncer-mini

The watcher uses `notify-debouncer-mini` for filesystem watching with built-in debouncing:

```rust
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

let mut debouncer = new_debouncer(
    Duration::from_millis(debounce_ms),
    move |result: DebounceEventResult| {
        // Handle events
    },
)?;
```

### Why Debouncing?

Editors use atomic save strategies that create brief temporary files:

1. Write to temporary file
2. Rename temp to target file

Without debouncing, a single save could trigger multiple events. The debounce window absorbs these into a single trigger.

### Event Filtering

Only `.rs` files trigger builds:

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

Changes to `Cargo.toml`, `index.html`, or other files are ignored.

## Watcher Lifetime

**Critical**: The `Debouncer` must be stored for the process lifetime.

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

Dropping the debouncer silently stops all file watching with no error or warning.

## Channel Communication

The watcher uses `tokio::sync::mpsc` for signaling:

```rust
pub fn start_watcher(
    watch_path: &Path,
    debounce_ms: u64,
    build_tx: Sender<()>,  // Tokio async channel
) -> Result<FileWatcher, notify::Error>
```

### blocking_send

The watcher callback runs on a background thread (notify's thread pool). To send to the async channel:

```rust
build_tx.blocking_send(())
```

This bridges the sync callback to the async channel.

## Configuration

```rust
let config = BuildConfig {
    watch_debounce_ms: 300,  // Default
    ..BuildConfig::new("../frontend")
};

let watcher = start_watcher(
    Path::new("../frontend/src"),
    config.watch_debounce_ms,
    build_tx,
)?;
```

### Debounce Recommendations

| Value | Use Case |
|-------|----------|
| 100-200ms | Fast feedback, may have duplicate builds |
| 300ms | Default, works well with most editors |
| 500-1000ms | Large projects, slower machines |

## Recursive Watching

The watcher monitors directories recursively:

```rust
debouncer.watcher().watch(
    watch_path,
    RecursiveMode::Recursive,
)?;
```

All subdirectories of `frontend/src/` are watched.

## Error Handling

Watcher errors are non-fatal:

```rust
Err(e) => {
    tracing::warn!("file watcher error: {}", e);
    // Continue watching
}
```

Common causes:
- Race condition on file deletion during atomic save
- Permission issues
- Path no longer exists

## Testing

```rust
#[test]
fn rs_file_change_triggers_build_signal() {
    let dir = tempdir().unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    
    let _watcher = start_watcher(dir.path(), 50, tx).unwrap();
    std::fs::write(dir.path().join("lib.rs"), b"// modified").unwrap();
    
    // Should receive signal
    assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok());
}

#[test]
fn non_rs_file_does_not_trigger_build_signal() {
    let dir = tempdir().unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    
    let _watcher = start_watcher(dir.path(), 50, tx).unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"text").unwrap();
    
    // Should NOT receive signal
    assert!(rx.recv_timeout(Duration::from_millis(600)).is_err());
}
```

## Platform Notes

The watcher uses `notify::RecommendedWatcher`:

- **Linux**: Uses inotify
- **macOS**: Uses FSEvents
- **Windows**: Uses ReadDirectoryChangesW

All platforms are supported transparently.
