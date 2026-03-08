# Dev Server

The wasm-drydock dev server owns the entire development loop — frontend builds, backend process management, file watching, live reload, and SCSS compilation — under a single command.

- [File Watchers](./watchers.md) — the five core watchers and how to configure custom watch paths
- [Hot Reload](./hot-reload.md) — how the browser is signalled to reload after a successful build
- [Error Overlay](./error-overlay.md) — how build failures are surfaced in the browser
- [SCSS Compilation](./scss.md) — in-memory SCSS compilation with the `grass` compiler
