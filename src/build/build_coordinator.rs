// src/build/build_coordinator.rs

//! Build coordinator for serializing rebuild requests.
//!
//! This module provides the build loop that receives file change signals
//! and executes builds in a serialized manner, coalescing rapid triggers
//! into at most two builds (one running + one pending).

use std::future::Future;
use tokio::sync::mpsc::Receiver;

use super::build::BuildError;

/// Run the build loop, serializing rebuild requests.
///
/// The loop waits for signals on `build_rx`, executes `build_fn`,
/// and coalesces rapid triggers into at most two builds (one running, one pending).
///
/// # Arguments
///
/// * `build_rx` - Channel receiver for build trigger signals
/// * `build_fn` - Async function to execute for each build
///
/// # Behavior
///
/// 1. Wait for a signal on `build_rx`
/// 2. Drain any queued messages and set `pending` flag if any were drained
/// 3. Execute `build_fn()`
/// 4. If `pending` is true, immediately start another build
/// 5. If `build_rx` is closed (sender dropped), exit the loop
///
/// # Error Handling
///
/// Build failures are logged at `warn!` level and do NOT exit the loop.
/// The server continues running and serving the last successful build.
/// Only channel closure exits the loop.
///
/// # Example
///
/// ```ignore
/// let (tx, rx) = tokio::sync::mpsc::channel(8);
///
/// tokio::spawn(async move {
///     run_build_loop(rx, || async {
///         run_wasm_pack(&config).await
///     }).await;
/// });
/// ```
pub async fn run_build_loop<F, Fut>(mut build_rx: Receiver<()>, build_fn: F)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), BuildError>> + Send,
{
    let mut pending = false;
    
    loop {
        // If not pending, wait for a trigger
        if !pending {
            match build_rx.recv().await {
                None => {
                    tracing::debug!("build channel closed — coordinator exiting");
                    return;
                }
                Some(()) => {}
            }
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
        let result = span.in_scope(|| build_fn()).await;

        match result {
            Ok(()) => {
                tracing::info!(parent: &span, "rebuild succeeded");
            }
            Err(ref e) => {
                // Build failure is NOT fatal to the loop — continue watching
                tracing::warn!(parent: &span, error = %e, "rebuild failed — previous assets still served");
            }
        }

        // If pending was set, loop will immediately start another build
        // (skipping the recv().await at the top)
        if pending {
            tracing::debug!("pending build trigger detected — starting next build immediately");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn single_trigger_causes_one_build() {
        let (tx, rx) = mpsc::channel(8);
        let count = Arc::new(Mutex::new(0u32));
        let c = count.clone();

        tx.send(()).await.unwrap();
        drop(tx); // Close channel to allow loop to exit

        run_build_loop(rx, move || {
            let c = c.clone();
            async move {
                *c.lock().unwrap() += 1;
                Ok(())
            }
        })
        .await;

        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn rapid_triggers_collapse_to_at_most_two_builds() {
        let (tx, rx) = mpsc::channel(8);
        let count = Arc::new(Mutex::new(0u32));
        let c = count.clone();

        // Send 5 rapid triggers
        for _ in 0..5 {
            tx.send(()).await.unwrap();
        }
        drop(tx); // Close channel to allow loop to exit

        run_build_loop(rx, move || {
            let c = c.clone();
            async move {
                *c.lock().unwrap() += 1;
                Ok(())
            }
        })
        .await;

        let final_count = *count.lock().unwrap();
        assert!(
            final_count <= 2,
            "expected at most 2 builds, got {final_count}"
        );
    }

    #[tokio::test]
    async fn build_failure_does_not_block_subsequent_builds() {
        let (tx, rx) = mpsc::channel(8);
        let count = Arc::new(Mutex::new(0u32));
        let first = Arc::new(Mutex::new(true));
        let c = count.clone();
        let f = first.clone();

        // Send 2 triggers
        for _ in 0..2 {
            tx.send(()).await.unwrap();
        }
        drop(tx); // Close channel to allow loop to exit

        run_build_loop(rx, move || {
            let c = c.clone();
            let f = f.clone();
            async move {
                *c.lock().unwrap() += 1;
                let mut is_first = f.lock().unwrap();
                if *is_first {
                    *is_first = false;
                    Err(BuildError::WasmPackFailed { exit_code: Some(1) })
                } else {
                    Ok(())
                }
            }
        })
        .await;

        assert_eq!(
            *count.lock().unwrap(),
            2,
            "both builds should have executed despite first failure"
        );
    }

    #[tokio::test]
    async fn closed_channel_exits_loop_cleanly() {
        let (tx, rx) = mpsc::channel::<()>(8);
        let count = Arc::new(Mutex::new(0u32));
        let c = count.clone();

        drop(tx); // Close immediately without sending

        run_build_loop(rx, move || {
            let c = c.clone();
            async move {
                *c.lock().unwrap() += 1;
                Ok(())
            }
        })
        .await;

        assert_eq!(
            *count.lock().unwrap(),
            0,
            "no builds should run when channel is closed"
        );
    }

    #[tokio::test]
    async fn builds_run_sequentially_not_concurrently() {
        let (tx, rx) = mpsc::channel(8);
        let concurrent_count = Arc::new(Mutex::new(0u32));
        let max_concurrent = Arc::new(Mutex::new(0u32));
        let cc = concurrent_count.clone();
        let mc = max_concurrent.clone();

        // Send multiple triggers
        for _ in 0..3 {
            tx.send(()).await.unwrap();
        }
        drop(tx);

        run_build_loop(rx, move || {
            let cc = cc.clone();
            let mc = mc.clone();
            async move {
                // Increment concurrent count
                {
                    let mut count = cc.lock().unwrap();
                    *count += 1;
                    let current = *count;
                    let mut max = mc.lock().unwrap();
                    if current > *max {
                        *max = current;
                    }
                }

                // Simulate some work
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                // Decrement concurrent count
                {
                    let mut count = cc.lock().unwrap();
                    *count -= 1;
                }

                Ok(())
            }
        })
        .await;

        let max = *max_concurrent.lock().unwrap();
        assert_eq!(
            max, 1,
            "builds should run sequentially, max concurrent was {max}"
        );
    }

    #[tokio::test]
    async fn pending_flag_triggers_immediate_rebuild() {
        let (tx, rx) = mpsc::channel(8);
        let build_times = Arc::new(Mutex::new(Vec::new()));
        let bt = build_times.clone();

        // Send trigger, wait a bit, send another
        tx.send(()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        tx.send(()).await.unwrap();
        drop(tx);

        run_build_loop(rx, move || {
            let bt = bt.clone();
            async move {
                bt.lock().unwrap().push(std::time::Instant::now());
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                Ok(())
            }
        })
        .await;

        let times = build_times.lock().unwrap();
        assert_eq!(times.len(), 2, "should have exactly 2 builds");

        // The second build should start immediately after the first completes
        // (within a small margin for the pending flag logic)
        if times.len() >= 2 {
            let gap = times[1].duration_since(times[0]);
            assert!(
                gap < std::time::Duration::from_millis(100),
                "second build should start immediately after first, gap was {:?}",
                gap
            );
        }
    }
}
