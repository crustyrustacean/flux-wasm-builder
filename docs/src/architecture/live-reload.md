# Live Reload

Live reload enables automatic browser refresh after successful frontend builds.

## Module

| File | Purpose |
|------|---------|
| [`reload.rs`](https://github.com/crustyrustacean/flux-wasm-builder/blob/main/src/build/reload.rs) | WebSocket reload signaling |

## Components

### 1. Reload Script

JavaScript injected into `index.html` at serve time:

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

**Features:**

- Uses `location.host` to work with any port
- Automatic reconnection on close (1 second delay)
- No explicit `onerror` handler needed — errors trigger `onclose`

### 2. Script Injection

The script is injected before `</body>`:

```rust
pub fn inject_reload_script(html: &str) -> String {
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + RELOAD_SCRIPT.len());
        result.push_str(&html[..pos]);
        result.push_str(RELOAD_SCRIPT);
        result.push_str(&html[pos..]);
        result
    } else {
        // No </body> found — append at end
        let mut result = String::with_capacity(html.len() + RELOAD_SCRIPT.len());
        result.push_str(html);
        result.push_str(RELOAD_SCRIPT);
        result
    }
}
```

**Note:** The script is NOT in `index.html` on disk. It's injected at serve time in dev mode.

### 3. WebSocket Handler

```rust
pub async fn ws_reload_handler(
    req: HttpRequest,
    body: web::Payload,
    reload_tx: web::Data<broadcast::Sender<()>>,
) -> Result<HttpResponse, actix_web::Error>
```

The handler:

1. Upgrades HTTP connection to WebSocket
2. Subscribes to the broadcast channel
3. Spawns a task that listens for reload signals
4. Sends "reload" text frame on signal

## Broadcast Channel

Reload signaling uses `tokio::sync::broadcast`:

```rust
let (reload_tx, _reload_rx) = broadcast::channel::<()>(16);
```

### Why broadcast?

- Multiple browser tabs can connect simultaneously
- Each tab gets its own subscription
- Non-blocking send (ignores "no receivers" error)

### Sending Reload Signal

After a successful build:

```rust
let _ = reload_tx.send(());
```

The `let _ =` is intentional: `send` returns `Err` when there are no active subscribers, which is expected when no browser tabs are open.

## WebSocket Task

```rust
actix_web::rt::spawn(async move {
    loop {
        tokio::select! {
            // Reload signal from build coordinator
            result = reload_rx.recv() => {
                match result {
                    Ok(()) => {
                        if session.text("reload").await.is_err() {
                            break;  // Client disconnected
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Subscriber fell behind — reload once
                        let _ = session.text("reload").await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;  // Channel closed
                    }
                }
            }
            // Client disconnected
            msg = msg_stream.next() => {
                match msg {
                    Some(Ok(Message::Close(reason))) => {
                        let _ = session.close(reason).await;
                        break;
                    }
                    None | Some(Err(_)) => {
                        break;
                    }
                    _ => {}  // Ignore other messages
                }
            }
        }
    }
});
```

## Route Registration

```rust
App::new()
    .app_data(web::Data::new(reload_tx))
    .route("/ws/reload", web::get().to(ws_reload_handler))
```

The WebSocket endpoint is only available in dev mode (compiled out with `embed-assets` feature).

## DevMode Flag

The `DevMode` struct controls script injection:

```rust
pub struct DevMode(pub bool);
```

In `main.rs`:

```rust
let dev_mode = DevMode(!cfg!(feature = "embed-assets"));

HttpServer::new(move || {
    App::new()
        .app_data(web::Data::new(dev_mode))
        // ...
})
```

## Reconnection Behavior

When the server restarts:

1. WebSocket connection closes
2. Browser's `onclose` handler triggers
3. After 1 second, browser reconnects
4. On next successful build, page reloads

This handles:

- Manual backend restarts
- Server crashes
- Network interruptions

## Testing

```rust
#[actix_web::test]
async fn ws_endpoint_returns_101_switching_protocols() {
    let (tx, _rx) = broadcast::channel::<()>(16);
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(tx))
            .route("/ws/reload", web::get().to(ws_reload_handler))
    ).await;
    
    let req = actix_test::TestRequest::get()
        .uri("/ws/reload")
        .insert_header(("upgrade", "websocket"))
        .insert_header(("connection", "upgrade"))
        .insert_header(("sec-websocket-version", "13"))
        .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
        .to_request();
    
    assert_eq!(actix_test::call_service(&app, req).await.status(), 101);
}
```

## Security Considerations

- WebSocket endpoint only accepts GET requests
- No authentication required (dev mode only)
- Only signals reload, no sensitive data transmitted
- Compiled out in production builds
