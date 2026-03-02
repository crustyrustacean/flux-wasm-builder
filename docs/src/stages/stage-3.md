# Stage 3: Live Reload

Stage 3 implements WebSocket-based live reload for automatic browser refresh.

## Overview

**Key Behavior:**
- WebSocket endpoint at `/ws/reload`
- Script injected into `index.html` at serve time
- Automatic reconnection on server restart
- Broadcast channel for multi-tab support

## Architecture

```
Browser                              Server
  │                                    │
  │  GET /ws/reload (WebSocket)        │
  │ ─────────────────────────────────► │
  │                                    │
  │  Subscribe to broadcast channel    │
  │                                    │
  │  <connected, waiting>              │
  │                                    │
  │         ┌── Build completes ──┐    │
  │         │                      │    │
  │  "reload" text frame           ◄────┤
  │ ◄───────────────────────────────   │
  │                                    │
  │  location.reload()                 │
  │                                    │
```

## Implementation Components

### RELOAD_SCRIPT

```javascript
<script>
(function() {
  function connect() {
    const ws = new WebSocket('ws://' + location.host + '/ws/reload');
    ws.onmessage = () => location.reload();
    ws.onclose = () => setTimeout(connect, 1000);
  }
  connect();
})();
</script>
```

Features:
- Uses `location.host` for any port
- Automatic reconnection (1 second delay)
- No explicit `onerror` needed (errors trigger `onclose`)

### inject_reload_script

```rust
pub fn inject_reload_script(html: &str) -> String
```

Injects the script immediately before `</body>`:

```rust
if let Some(pos) = html.rfind("</body>") {
    // Insert before </body>
} else {
    // Append at end
}
```

### ws_reload_handler

```rust
pub async fn ws_reload_handler(
    req: HttpRequest,
    body: web::Payload,
    reload_tx: web::Data<broadcast::Sender<()>>,
) -> Result<HttpResponse, actix_web::Error>
```

Handles WebSocket upgrade and subscribes to broadcast channel.

## Broadcast Channel

```rust
let (reload_tx, _) = broadcast::channel::<()>(16);
```

### Why broadcast?

- Multiple browser tabs can connect
- Each tab gets its own subscription
- Non-blocking send (ignores "no receivers" error)

### Sending Reload Signal

```rust
let _ = reload_tx.send(());
```

The `let _ =` is intentional: `send` returns `Err` when there are no subscribers, which is expected when no browser tabs are open.

## WebSocket Task

```rust
tokio::select! {
    // Reload signal
    result = reload_rx.recv() => {
        match result {
            Ok(()) => {
                session.text("reload").await?;
            }
            Err(Lagged(n)) => {
                // Subscriber fell behind — reload once
                session.text("reload").await?;
            }
            Err(Closed) => break,
        }
    }
    // Client disconnect
    msg = msg_stream.next() => {
        // Handle close frame or disconnect
    }
}
```

## DevMode Flag

```rust
pub struct DevMode(pub bool);
```

Controls script injection:
- `DevMode(true)`: Inject reload script
- `DevMode(false)`: Serve index.html as-is

## Test Coverage

- Script injected before `</body>`
- Handles HTML without body tag
- Script references `/ws/reload`
- Script contains `onclose` reconnect
- Original content preserved
- WebSocket endpoint returns 101
- Broadcast send with no receivers doesn't panic

## Route Registration

```rust
App::new()
    .app_data(web::Data::new(reload_tx))
    .route("/ws/reload", web::get().to(ws_reload_handler))
```

The WebSocket endpoint is compiled out when `embed-assets` feature is active.
