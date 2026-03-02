# Development Mode

Development mode is the default runtime mode for flux-wasm-builder generated projects. It provides a rich development experience with automatic rebuilding and live reload.

## Starting Development

```bash
cd my-app
cargo backend
```

This single command:

1. **Builds the frontend** using `wasm-pack build --target web --dev`
2. **Starts the Actix-web server** on port 8080
3. **Watches for changes** in `frontend/src/**/*.rs`
4. **Triggers rebuilds** when Rust source files change
5. **Signals browser reload** after successful builds

## Development Workflow

### File Watching

The file watcher monitors `frontend/src/` recursively for `.rs` file changes:

- **Debounce interval**: 300ms (configurable)
- **File types**: Only `.rs` files trigger rebuilds
- **Atomic saves**: Handled correctly by debouncing

When a change is detected:

1. The build coordinator receives a signal
2. Rapid changes are coalesced into at most two builds
3. `wasm-pack` rebuilds the frontend
4. On success, connected browsers receive a reload signal

### Live Reload

The development server injects a WebSocket client into `index.html` at serve time:

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

- **Automatic reconnection**: If the server restarts, the browser reconnects
- **No manual refresh**: Page reloads automatically after successful builds
- **Multiple tabs**: All connected tabs reload simultaneously

### Build Coordination

The build coordinator ensures efficient rebuild behavior:

- **Serialized builds**: Only one build runs at a time
- **Coalescing**: Rapid file saves trigger at most two builds
- **Error tolerance**: Failed builds don't crash the server
- **Previous assets served**: On build failure, the last successful build remains available

## Request Routing

In development mode, the server handles requests in this order:

| Route | Handler | Description |
|-------|---------|-------------|
| `/api/*` | Application handlers | Your API routes (registered first) |
| `/pkg/{filename}` | Static asset handler | WASM, JS, and build artifacts |
| `/ws/reload` | WebSocket handler | Live reload signaling |
| `*` (fallback) | SPA handler | Returns `index.html` with reload script |

## Development vs Production

| Feature | Development | Production (`embed-assets`) |
|---------|-------------|---------------------------|
| wasm-pack invocation | On startup and changes | None (pre-built) |
| File watcher | Active | Compiled out |
| WebSocket endpoint | Available | Compiled out |
| Reload script | Injected into HTML | Not injected |
| Asset serving | From filesystem | From embedded bytes |
| Build timeout | 300 seconds | N/A |

## Handling Build Failures

When a build fails:

1. The error is logged to the terminal
2. The server continues running
3. Previous assets remain available
4. The browser shows the last successful state
5. Fix the error and save to trigger a new build

Build failures do **not** crash the server or disconnect browsers.

## Terminal Output

Development mode streams `wasm-pack` output directly to the terminal:

```
[INFO] starting wasm-pack build
[INFO] wasm-pack build completed successfully
[INFO] Actix-web server running at http://localhost:8080
[INFO] file watcher active on frontend/src/
[DEBUG] Rust source change detected
[INFO] rebuild succeeded
```

Compiler errors appear inline with full diagnostics.

## Next Steps

- Learn about [Production Builds](./production-builds.md)
- Understand [Configuration Options](./configuration.md)
- Read about [Live Reload Architecture](../architecture/live-reload.md)
