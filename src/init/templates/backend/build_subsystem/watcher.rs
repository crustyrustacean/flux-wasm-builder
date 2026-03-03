use std::path::Path;
use std::time::Duration;

use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use tokio::sync::mpsc::Sender;

/// Type alias for the watcher type returned by [`start_watcher`].
pub type FileWatcher = notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>;

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
) -> Result<FileWatcher, notify_debouncer_mini::notify::Error> {
    let span = tracing::info_span!(
        "file_watcher",
        path = %watch_path.display(),
        debounce_ms
    );
    let _enter = span.enter();

    tracing::info!("starting file watcher");

    let mut debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    let rs_change = events.iter().any(|e| {
                        e.path.extension().map(|ext| ext == "rs").unwrap_or(false)
                    });

                    if rs_change {
                        tracing::debug!(event_count = events.len(), "Rust source change detected");
                        // Use blocking_send() because this callback runs on a non-async
                        // background thread (notify's thread pool), but the receiver is
                        // a tokio async channel.
                        if let Err(e) = build_tx.blocking_send(()) {
                            tracing::debug!(error = %e, "build channel closed — watcher callback exiting");
                        }
                    } else {
                        tracing::trace!("ignored non-.rs file event");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "file watcher error");
                }
            }
        },
    )?;

    debouncer.watcher().watch(watch_path, RecursiveMode::Recursive)?;
    tracing::info!("file watcher active");

    Ok(debouncer)
}