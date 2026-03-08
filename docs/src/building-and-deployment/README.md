# Building & Deployment

wasm-drydock produces a single self-contained binary that embeds all frontend assets. This makes deployment straightforward — there is no separate static file server to manage.

- [Release Binary](./release-binary.md) — build a release binary with `wasm-drydock release`
- [Docker](./docker.md) — build a minimal production image using the generated `Dockerfile`
- [Fly.io](./fly-io.md) — deploy to Fly.io with `fly deploy`
