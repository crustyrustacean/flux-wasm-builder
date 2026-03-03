use std::future::Future;
use tokio::sync::{mpsc::Receiver, broadcast};

use super::build::BuildError;

/// Run the build loop, serializing rebuild requests.
///
/// The loop waits for signals on `build_rx`, executes `build_fn`,
/// and coalesces rapid triggers into at most two builds (one running, one pending).
/// After a successful build, sends a reload signal via `reload_tx`.
pub async fn run_build_loop<F, Fut>(
    mut build_rx: Receiver<()>,
    reload_tx: broadcast::Sender<()>,
    build_fn: F,
)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), BuildError>> + Send,
{
    let mut pending = false;
    
    loop {
        // If not pending, wait for a trigger
        if !pending && build_rx.recv().await.is_none() {
            tracing::debug!("build channel closed — coordinator exiting");
            return;
        }
        
        // Reset pending flag - we're now processing this build
        pending = false;

        // Drain any queued messages and set pending flag for next iteration
        while build_rx.try_recv().is_ok() {
            pending = true;
        }
        if pending {
            tracing::debug!("coalesced rapid triggers — running one build with one queued");
        }

        // Execute build
        let span = tracing::info_span!("rebuild_cycle");
        let result = span.in_scope(&build_fn).await;

        match result {
            Ok(()) => {
                tracing::info!(parent: &span, "rebuild succeeded");
                // Send reload signal to connected browsers.
                // The `let _ =` is intentional: send returns Err when there
                // are no active subscribers, which is expected when no browser
                // tabs are open. This must not be treated as an error.
                let _ = reload_tx.send(());
            }
            Err(ref e) => {
                tracing::warn!(parent: &span, error = %e, "rebuild failed — previous assets still served");
            }
        }

        if pending {
            tracing::debug!("pending build trigger detected — starting next build immediately");
        }
    }
}