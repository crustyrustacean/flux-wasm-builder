# File Watchers

## Core watchers

Five core watchers run at all times:

| Watcher | Path | Filters | On change |
|---------|------|---------|-----------|
| Frontend | `frontend/src/` | `.rs` | `wasm-pack build`, then browser reload |
| Backend | `backend/src/` | `.rs` | Kill backend, respawn, health check, then browser reload |
| Styles | `frontend/styles/` | `.scss` | Recompile SCSS in memory, browser reload |
| Public assets | `frontend/public/` | any file | Browser reload |
| HTML | `frontend/` | `.html` | Browser reload |

## Custom watch paths

Additional watch paths can be configured in `drydock.toml` using `[[watch]]` entries:

```toml
[[watch]]
path = "frontend/content"
extensions = ["md", "mdx"]
action = "reload"

[[watch]]
path = "frontend/assets"
extensions = []
action = "reload"

[[watch]]
path = "shared/src"
extensions = ["rs"]
action = "rebuild"
```

### Available actions

| Action | Effect |
|--------|--------|
| `reload` | Trigger browser page reload |
| `rebuild` | Trigger WASM rebuild then reload |
| `restart` | Trigger backend restart |

### Notes

- Paths are relative to the project root
- Empty `extensions` array watches all files
- Non-existent paths are logged as warnings and skipped
