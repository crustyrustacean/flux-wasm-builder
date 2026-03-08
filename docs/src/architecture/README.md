# Architecture

This section covers the internal design of the wasm-drydock dev server.

- [Build Coordinator](./build-coordinator.md) — serializes rebuild requests and coalesces rapid file changes
- [Process Manager](./process-manager.md) — owns the backend child process and handles restarts
- [Reload Channel](./reload-channel.md) — broadcasts reload and error messages to connected browser tabs
- [Static Assets](./static-assets.md) — how assets are served differently in development and release modes
