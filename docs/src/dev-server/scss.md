# SCSS Compilation

The dev server compiles `frontend/styles/screen.scss` using the `grass` SCSS compiler and serves the result at `/styles/screen.css`.

Compilation happens:

- On startup
- Whenever a `.scss` file in `frontend/styles/` changes

The compiled CSS is held in memory and never written to disk during development. `frontend/styles/screen.css` is listed in `.gitignore` for this reason.

If SCSS compilation fails at startup, a warning is logged and the server continues with an empty stylesheet. The error overlay will display the compiler message on subsequent failures during the watch cycle.

## Release builds

`wasm-drydock release` writes `frontend/styles/screen.css` to disk so it can be embedded into the release binary.
