# Process Manager

`src/dev/mod.rs`

The `ProcessManager` struct owns the backend child process for the lifetime of the dev server.

## Lifecycle

On `new()`, the backend is spawned via `tokio::process::Command` with `kill_on_drop(true)`. This means the backend process is automatically killed if `ProcessManager` is dropped for any reason — clean exit, panic, or error propagation.

## Restart

When a change is detected in `backend/src/`, `ProcessManager::restart()` is called:

1. The existing child process is taken from the `Option`, killed, and awaited
2. A new backend process is spawned
3. The dev server polls `/api/health_check` until the new backend is ready
4. A reload signal is sent to connected browsers

Both `kill()` and `wait()` are async operations using `tokio::process::Child`, so they yield properly and do not block the Tokio runtime.

## Graceful shutdown

`kill_on_drop(true)` on `tokio::process::Child` ensures the backend process is cleaned up on any exit path without requiring an explicit `Drop` implementation.
