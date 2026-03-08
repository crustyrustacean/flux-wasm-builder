# Backend

The scaffolded backend is an opinionated Actix-web starter. It is structured around a small set of focused modules:

- **`configuration.rs`** — YAML-based configuration loading. Walks up the directory tree to find `configuration/base.yaml`. Respects the `DRYDOCK_BACKEND_PORT` environment variable set by the dev server.
- **`startup.rs`** — Wires up the Actix-web server, routes, and middleware.
- **`error.rs`** — `ApiError` enum implementing `ResponseError`, mapping variants to HTTP status codes.
- **`response.rs`** — Re-exports `ApiResponse<T>` from the shared crate for use in handlers.
- **`telemetry.rs`** — Tracing subscriber setup with Bunyan-formatted JSON output.
- **`static_assets.rs`** — Serves embedded frontend assets in release builds. Compiled out in development.
- **`api/mod.rs`** — Route configuration with starter endpoints: `/api/health_check`, `/api/hello`, `/api/status`.

## Integration tests

Integration test scaffolding lives in `backend/tests/api/` with a `TestApp` helper that spawns the server on a random port, plus a sample health check test to build on.

## Feature flags

| Flag | Effect |
|------|--------|
| `embed-assets` | Embeds all frontend assets and `configuration/base.yaml` into the binary at compile time |
