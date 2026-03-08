# Build Coordinator

`src/build/build_coordinator.rs`

The build coordinator serializes rebuild requests, ensuring builds never run concurrently and that rapid file changes don't trigger redundant builds.

## Coalescing

The coordinator implements a one-pending-build policy:

- If a build is running and a new trigger arrives, the trigger is noted as pending
- When the running build finishes, the pending build starts immediately
- Additional triggers that arrive while a build is running and one is already pending are discarded

This means at most two builds are ever queued: one running and one pending.

## Error handling

Build failures are logged at `warn` level but do not exit the loop. The server continues running and serving the last successful build. Only channel closure exits the loop.

## Reload signaling

After a successful build, a `DevServerMessage::Reload` is sent on the broadcast channel. After a failed build, a `DevServerMessage::BuildError` is sent, which the browser displays as an error overlay.
