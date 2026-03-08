# Release Binary

```bash
wasm-drydock release
```

This command:

1. Runs `wasm-pack build --release` on the frontend
2. Compiles `frontend/styles/screen.scss` to `frontend/styles/screen.css`
3. Runs `cargo build --release --features embed-assets` on the backend

The result is a single self-contained binary in `target/release/` with all frontend assets — WASM, JavaScript, CSS, `index.html`, and public assets — compiled in via the `embed-assets` feature flag.

## What is embedded

| Asset | Source | Mechanism |
|-------|--------|-----------|
| WASM and JS | `frontend/pkg/` | `include_dir!` |
| HTML | `frontend/index.html` | `include_bytes!` |
| CSS | `frontend/styles/screen.css` | `include_bytes!` |
| Public files | `frontend/public/` | `include_dir!` |
| Configuration | `configuration/base.yaml` | `include_str!` |

The binary has zero runtime filesystem dependencies.

## Running the release binary

```bash
./target/release/my-app-backend
```

## Configuration at runtime

The release binary embeds `configuration/base.yaml` at compile time. To override settings at runtime without recompiling, place an environment-specific YAML file next to the binary:

```
my-app-backend          ← the binary
configuration/
└── production.yaml     ← optional override
```

`APP_*` environment variables are always applied last and override everything:

```bash
APP_APPLICATION__HOST=0.0.0.0 ./my-app-backend
```
