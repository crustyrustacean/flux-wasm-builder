// tests/watcher.rs
//
// Integration tests for the file watcher and build coordinator.
// Run with: cargo test --test watcher

use std::time::Duration;

use flux_wasm_builder::{BuildError, run_build_loop, start_watcher};
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

// =============================================================================
// Watcher Integration Tests
// =============================================================================

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

    // Watch the `src` subdirectory so is_src_dir = true (only .rs files trigger)
    let watcher = start_watcher(&src, 50, tx).expect("watcher should start");

    // Wait for watcher to initialize
    std::thread::sleep(Duration::from_millis(200));

    // Drop the watcher
    drop(watcher);

    // Wait for the watcher background thread to fully terminate so tx is dropped
    std::thread::sleep(Duration::from_millis(500));

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

// =============================================================================
// Build Coordinator Integration Tests
// =============================================================================

#[tokio::test]
async fn single_trigger_causes_one_build() {
    let (tx, rx) = mpsc::channel(8);
    let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);
    let count = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let c = count.clone();

    tx.send(()).await.unwrap();
    drop(tx); // Close channel to allow loop to exit

    run_build_loop(rx, reload_tx, move || {
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
    let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);
    let count = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let c = count.clone();

    // Send 5 rapid triggers
    for _ in 0..5 {
        tx.send(()).await.unwrap();
    }
    drop(tx); // Close channel to allow loop to exit

    run_build_loop(rx, reload_tx, move || {
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
    let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);
    let count = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let first = std::sync::Arc::new(std::sync::Mutex::new(true));
    let c = count.clone();
    let f = first.clone();

    // Send 2 triggers
    for _ in 0..2 {
        tx.send(()).await.unwrap();
    }
    drop(tx); // Close channel to allow loop to exit

    run_build_loop(rx, reload_tx, move || {
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
    let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);
    let count = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let c = count.clone();

    drop(tx); // Close immediately without sending

    run_build_loop(rx, reload_tx, move || {
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

// =============================================================================
// Combined Watcher + Coordinator Tests
// =============================================================================

#[tokio::test]
async fn watcher_and_coordinator_integration() {
    // This test verifies that the watcher can signal the coordinator
    // through the tokio::sync::mpsc channel using blocking_send

    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("lib.rs"), b"// initial").unwrap();

    // Create the tokio async channel
    let (tx, mut rx) = mpsc::channel(8);

    // Start the watcher with the tokio sender
    // The watcher will use blocking_send() internally
    let _watcher = start_watcher(dir.path(), 50, tx).expect("watcher should start");

    // Wait for watcher to initialize
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Track build count
    let build_count = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let bc = build_count.clone();

    // Spawn the build loop
    let build_handle = tokio::spawn(async move {
        while let Some(()) = rx.recv().await {
            *bc.lock().unwrap() += 1;
            if *bc.lock().unwrap() >= 1 {
                break; // Exit after one build for this test
            }
        }
    });

    // Trigger a file change
    std::fs::write(src.join("lib.rs"), b"// modified").unwrap();

    // Wait for the build to complete
    let result = tokio::time::timeout(Duration::from_secs(5), build_handle).await;

    assert!(result.is_ok(), "build should have completed within timeout");
    assert_eq!(
        *build_count.lock().unwrap(),
        1,
        "should have exactly one build"
    );
}
