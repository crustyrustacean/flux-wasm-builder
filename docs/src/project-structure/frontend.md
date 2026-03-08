# Frontend

The frontend is a Yew CSR application compiled to WebAssembly via wasm-pack.

## Entry point

`frontend/src/lib.rs` contains the root `App` component and the `#[wasm_bindgen(start)]` entry point that initialises the logger, panic hook, and Yew renderer.

## Styles

SCSS lives in `frontend/styles/screen.scss`. The dev server compiles it to CSS in memory and serves it at `/styles/screen.css`. It is never written to disk during development — the compiled `screen.css` is only produced by `wasm-drydock release`.

A CSS reset based on Josh Comeau's modern reset is included by default.

## Public assets

Static files placed in `frontend/public/` are served as-is at their filename. For example, `frontend/public/favicon.png` is served at `/favicon.png`.

## index.html

`frontend/index.html` bootstraps the application. The WASM module is loaded via an ES module script:

```html
<script type="module">
    import init from '/pkg/my_app_frontend.js';
    init();
</script>
```

Note the absolute path — relative paths break on SPA subroutes.
