# Hot Reload

The dev server injects a small JavaScript snippet into `index.html` before serving it in development mode. This snippet establishes a WebSocket connection to `/ws/reload`.

When a build or reload event occurs, the server sends a message over the WebSocket:

- `"reload"` — the browser reloads the page
- `"error:<message>"` — the browser displays the error overlay

The snippet reconnects automatically if the WebSocket connection drops, with a one second delay. This means the browser reconnects seamlessly if the dev server restarts.

The reload script is never present in release builds.
