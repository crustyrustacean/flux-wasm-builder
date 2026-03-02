# Quick Start

This guide walks you through creating your first fullstack Rust web application with flux-wasm-builder.

## Step 1: Create a New Project

```bash
flux-wasm-builder init my-app
```

This creates a three-crate Cargo workspace:

```
my-app/
├── .cargo/config.toml      # cargo alias
├── .gitignore
├── Cargo.toml              # workspace root
├── backend/                # Actix-web server
├── frontend/               # Yew WASM application
└── shared/                 # API types
```

## Step 2: Navigate to Your Project

```bash
cd my-app
```

## Step 3: Start Development Server

```bash
cargo backend
```

This command:

1. Builds the frontend with `wasm-pack`
2. Starts the Actix-web server on `http://localhost:8080`
3. Watches for frontend source changes
4. Triggers rebuilds and browser reloads automatically

## Step 4: Open in Browser

Navigate to `http://localhost:8080` in your browser. You should see:

- A Yew frontend application
- A "Hello" message fetched from `/api/hello`
- A status display from `/api/status`

## Step 5: Make Changes

Edit `frontend/src/lib.rs` to modify the frontend. Save the file and watch the browser automatically reload with your changes.

## Step 6: Production Build

When ready to deploy:

```bash
# Build frontend for release
wasm-pack build frontend/ --target web --release

# Build backend with embedded assets
cargo build --release --features embed-assets
```

The resulting binary in `target/release/` is self-contained and ready for deployment.

## Next Steps

- Read about [Project Scaffolding](../user-guide/scaffolding.md) to understand the generated structure
- Learn about [Development Mode](../user-guide/development-mode.md) features
- Configure your project with [Configuration Options](../user-guide/configuration.md)
