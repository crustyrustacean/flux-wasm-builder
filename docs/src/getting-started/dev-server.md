# The Dev Server

## Starting the server

From your project root:

```bash
wasm-drydock dev
```

On startup the dev server:

1. Compiles `frontend/styles/screen.scss` to CSS
2. Spawns the backend process and polls `/api/health_check` until it responds
3. Runs an initial `wasm-pack build` on the frontend
4. Starts the HTTP server on `http://localhost:8080`

## Opening the browser automatically

```bash
wasm-drydock dev --open
```

## What the server handles

| Path | Behaviour |
|------|-----------|
| `/api/*` | Proxied to the backend |
| `/pkg/*` | Serves wasm-pack build output |
| `/styles/screen.css` | Serves compiled SCSS |
| `/ws/reload` | WebSocket live reload endpoint |
| `/*` | SPA fallback — serves `index.html` |

## File watchers

The dev server runs five core watchers simultaneously:

| Watcher | Path | Filters | On change |
|---------|------|---------|-----------|
| Frontend | `frontend/src/` | `.rs` | `wasm-pack build`, then browser reload |
| Backend | `backend/src/` | `.rs` | Kill backend, respawn, health check, then browser reload |
| Styles | `frontend/styles/` | `.scss` | Recompile SCSS in memory, browser reload |
| Public assets | `frontend/public/` | any file | Browser reload |
| HTML | `frontend/` | `.html` | Browser reload |

See [File Watchers](../dev-server/watchers.md) for custom watch path configuration.

## Error overlay

When a build fails, the error is pushed to the browser via WebSocket and displayed as a full-screen overlay. The overlay is dismissed automatically when the next successful build completes.
