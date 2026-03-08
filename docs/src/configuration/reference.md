# Configuration Reference

`drydock.toml` lives at the workspace root. All `[dev]` fields are optional and fall back to the defaults shown.

```toml
[project]
name = "my-app"

[dev]
public_port = 8080          # browser-facing port
backend_port = 3001         # internal backend port
watch_debounce_ms = 300     # file change debounce interval in milliseconds
```

## `[project]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | The project name. Used to identify the backend binary. |

## `[dev]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `public_port` | integer | `8080` | The port the dev server binds to |
| `backend_port` | integer | `3001` | The port the backend process binds to |
| `watch_debounce_ms` | integer | `300` | Milliseconds to wait after a file change before triggering a rebuild |

## `[[watch]]`

Zero or more custom watch entries. Each entry has the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | Path to watch, relative to the project root |
| `extensions` | array of strings | no | File extensions to filter. Empty array watches all files. |
| `action` | string | yes | One of `"reload"`, `"rebuild"`, `"restart"` |

## Environment variables

The backend respects the following environment variables at runtime:

| Variable | Description |
|----------|-------------|
| `DRYDOCK_BACKEND_PORT` | Overrides the backend port. Set automatically by the dev server. |
| `APP_ENVIRONMENT` | Selects the environment config file (`local` or `production`). Defaults to `local`. |
| `APP_APPLICATION__HOST` | Overrides the bind host. Use `0.0.0.0` for production deployments. |
| `APP_APPLICATION__PORT` | Overrides the bind port. |
