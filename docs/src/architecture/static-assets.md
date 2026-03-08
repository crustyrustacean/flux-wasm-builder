# Static Assets

Static asset serving behaves differently in development and release modes.

## Development mode

Assets are served from the filesystem at runtime:

- `/pkg/*` — files from `frontend/pkg/` (wasm-pack output)
- `/styles/screen.css` — CSS compiled from SCSS, held in memory
- `/favicon.png` and other public files — files from `frontend/public/`
- `/*` — SPA fallback serves `frontend/index.html` with the reload script injected

## Release mode (`embed-assets` feature)

All assets are embedded into the binary at compile time:

- `frontend/pkg/` — embedded via `include_dir!`
- `frontend/index.html` — embedded via `include_bytes!`
- `frontend/styles/screen.css` — embedded via `include_bytes!`
- `frontend/public/` — embedded via `include_dir!`
- `configuration/base.yaml` — embedded via `include_str!`

No filesystem access is required at runtime. The binary is fully self-contained.

## Path traversal protection

All asset handlers extract only the final path component via `Path::file_name()` before serving. This prevents path traversal attacks such as `/pkg/../../etc/passwd`.
