// src/build/watcher.rs

//! File watcher for detecting Rust source changes.
//!
//! This module provides filesystem watching functionality that monitors
//! the frontend source directory and triggers rebuilds when `.rs` files
//! are modified.

use std::path::Path;
use std::time::Duration;

use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use tokio::sync::mpsc::Sender;

/// Type alias for the watcher type returned by [`start_watcher`].
/// Uses the recommended watcher backend for the current platform.
pub type FileWatcher =
    notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>;

/// Start the file watcher.
///
/// # Important — Lifetime
/// The returned `Debouncer` **must be stored** for the lifetime of the process.
/// Dropping it silently stops all file watching with no error or warning.
/// Store it in a long-lived struct field, never in a local variable.
///
/// # Arguments
///
/// * `watch_path` - The directory to watch recursively (e.g., `frontend/src/`)
/// * `debounce_ms` - Debounce interval in milliseconds (recommended: 300)
/// * `build_tx` - Tokio async channel sender to notify when a rebuild is needed
///
/// # Returns
///
/// Returns a `Debouncer` on success, or a `notify::Error` on failure.
///
/// # Channel Type
///
/// The `build_tx` parameter is a `tokio::sync::mpsc::Sender` which is an async channel.
/// The watcher callback runs on a background thread managed by notify, so it uses
/// `blocking_send()` to send to the async channel from this non-async context.
///
/// # Example
///
/// ```ignore
/// let (tx, rx) = tokio::sync::mpsc::channel(8);
/// let watcher = start_watcher(Path::new("../frontend/src"), 300, tx)?;
/// // Store `watcher` in a long-lived struct - do not drop it!
/// ```
pub fn start_watcher(
    watch_path: &Path,
    debounce_ms: u64,
    build_tx: Sender<()>,
) -> Result<FileWatcher, notify_debouncer_mini::notify::Error> {
    let span = tracing::info_span!(
        "file_watcher",
        path = %watch_path.display(),
        debounce_ms
    );
    let _enter = span.enter();

    tracing::info!("starting file watcher");

    // For Rust source directories, only trigger on .rs changes.
    // For other directories (e.g. public/), trigger on any file change.
    let is_src_dir = watch_path.ends_with("src");

    let mut debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    let should_trigger = if is_src_dir {
                        events
                            .iter()
                            .any(|e| e.path.extension().map(|ext| ext == "rs").unwrap_or(false))
                    } else {
                        !events.is_empty()
                    };

                    if should_trigger {
                        tracing::debug!(event_count = events.len(), "file change detected");
                        // Use blocking_send() because this callback runs on a non-async
                        // background thread (notify's thread pool), but the receiver is
                        // a tokio async channel.
                        if let Err(e) = build_tx.blocking_send(()) {
                            // The receiver has been dropped — the build coordinator
                            // has exited. Stop logging; the process is likely shutting down.
                            tracing::debug!(error = %e, "build channel closed — watcher callback exiting");
                        }
                    } else {
                        tracing::trace!("ignored non-.rs file event");
                    }
                }
                Err(e) => {
                    // Watcher errors are non-fatal — log and continue.
                    // Common causes: race condition on file deletion during atomic save.
                    tracing::warn!(error = %e, "file watcher error");
                }
            }
        },
    )?;

    debouncer
        .watcher()
        .watch(watch_path, RecursiveMode::Recursive)?;
    tracing::info!("file watcher active");

    Ok(debouncer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    // Note: The watcher callback runs on a background thread (notify's thread pool).
    // It uses blocking_send() to send to the tokio async channel.
    // Tests use timeouts with recv() to avoid hanging indefinitely.

    #[test]
    fn rs_file_change_triggers_build_signal() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("lib.rs"), b"// initial").unwrap();

        let (tx, mut rx) = mpsc::channel(8);
        let _watcher = start_watcher(dir.path(), 50, tx).expect("watcher should start");

        // Wait for watcher to initialize
        std::thread::sleep(Duration::from_millis(200));

        // Modify a .rs file
        std::fs::write(src.join("lib.rs"), b"// modified").unwrap();

        // Use a timeout to avoid hanging forever
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { tokio::time::timeout(Duration::from_secs(3), rx.recv()).await })
        })
        .join()
        .unwrap();

        assert!(
            result.is_ok() && result.unwrap().is_some(),
            "should receive build signal after .rs file change"
        );
    }

    #[test]
    fn non_rs_file_does_not_trigger_build_signal() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();

        let (tx, mut rx) = mpsc::channel(8);
        // Watch the `src` subdirectory so is_src_dir = true (only .rs files trigger)
        let _watcher = start_watcher(&src, 50, tx).expect("watcher should start");

        // Wait for watcher to initialize
        std::thread::sleep(Duration::from_millis(200));

        // Modify a non-.rs file
        std::fs::write(src.join("notes.txt"), b"not rust").unwrap();

        // Use a timeout - should timeout since no signal should come
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { tokio::time::timeout(Duration::from_millis(600), rx.recv()).await })
        })
        .join()
        .unwrap();

        assert!(
            result.is_err(),
            "non-.rs change should not trigger a build signal (should timeout)"
        );
    }

    #[test]
    fn dropping_watcher_stops_events() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("lib.rs"), b"// initial").unwrap();

        let (tx, mut rx) = mpsc::channel(8);

        let watcher = start_watcher(&src, 50, tx).expect("watcher should start");

        // Wait for watcher to initialize
        std::thread::sleep(Duration::from_millis(200));

        // Drop the watcher
        drop(watcher);

        // Wait a bit for cleanup
        std::thread::sleep(Duration::from_millis(100));

        // Modify a file
        std::fs::write(src.join("lib.rs"), b"// change after drop").unwrap();

        // Use a timeout - should timeout or get None since watcher is dropped
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { tokio::time::timeout(Duration::from_millis(600), rx.recv()).await })
        })
        .join()
        .unwrap();

        // Either timeout (no signal) or receive None (channel closed)
        assert!(
            result.is_err() || result.unwrap().is_none(),
            "should receive no signal after watcher is dropped"
        );
    }

    #[test]
    fn watcher_returns_error_for_nonexistent_path() {
        let (tx, _rx) = mpsc::channel(8);
        let result = start_watcher(Path::new("/nonexistent/path/that/does/not/exist"), 50, tx);
        assert!(result.is_err(), "should return error for nonexistent path");
    }

    #[test]
    fn rapid_changes_are_debounced() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("lib.rs"), b"// initial").unwrap();

        let (tx, mut rx) = mpsc::channel(8);
        let _watcher = start_watcher(&src, 100, tx).expect("watcher should start");

        // Wait for watcher to initialize
        std::thread::sleep(Duration::from_millis(200));

        // Make rapid changes
        for i in 0..5 {
            std::fs::write(src.join("lib.rs"), format!("// change {i}").as_bytes()).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }

        // Due to debouncing, we should receive at most 1-2 signals
        // Wait a bit for all events to be processed
        std::thread::sleep(Duration::from_millis(300));

        // Count messages without blocking
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }

        // With 100ms debounce and 5 changes over 100ms, we expect at most 2 signals
        assert!(
            count <= 2,
            "rapid changes should be debounced, got {count} signals"
        );
    }
}
