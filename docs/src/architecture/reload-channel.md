# Reload Channel

`src/build/reload.rs`

The reload channel is a `tokio::sync::broadcast` channel that carries `DevServerMessage` values from the server to all connected browser tabs.

## Message types

```rust
pub enum DevServerMessage {
    Reload,
    BuildError(String),
}
```

## Flow

1. A watcher detects a file change and sends a trigger to the build coordinator
2. The build coordinator runs the build
3. On success, it sends `DevServerMessage::Reload` on the broadcast channel
4. On failure, it sends `DevServerMessage::BuildError(message)`
5. The WebSocket handler receives the message and forwards it to the browser
6. The browser either reloads or displays the error overlay

## WebSocket handler

`ws_reload_handler` upgrades the HTTP connection to WebSocket and subscribes to the broadcast channel. It handles:

- `Reload` — sends `"reload"` text frame
- `BuildError` — sends `"error:<message>"` text frame
- `RecvError::Lagged` — subscriber fell behind, sends one reload
- `RecvError::Closed` — broadcast channel closed, exits loop
- Client disconnect — exits loop cleanly
