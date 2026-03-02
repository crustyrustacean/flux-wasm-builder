# Production Builds

Production mode creates a self-contained binary with embedded WebAssembly assets, ready for deployment.

## Build Process

### Step 1: Build Frontend

```bash
wasm-pack build frontend/ --target web --release
```

This creates optimized WASM output in `frontend/pkg/`:

```
frontend/pkg/
├── my_app_frontend.js       # JavaScript glue code
├── my_app_frontend_bg.wasm  # WebAssembly binary
├── my_app_frontend.d.ts     # TypeScript declarations
└── package.json             # Package metadata
```

### Step 2: Build Backend with Embedded Assets

```bash
cargo build --release --features embed-assets
```

This produces a single binary in `target/release/` that contains:

- The Actix-web server
- Embedded WASM binary
- Embedded JavaScript glue code
- Embedded `index.html`

## Feature Flag: embed-assets

The `embed-assets` feature triggers compile-time asset embedding:

```toml
[features]
embed-assets = ["dep:include_dir"]
```

When active:

| Component | Behavior |
|-----------|----------|
| File watcher | Compiled out |
| WebSocket endpoint | Compiled out |
| Reload script injection | Disabled |
| Asset serving | From embedded bytes |

## Build Guard

The `build.rs` script validates that assets exist before compilation:

```rust
fn main() {
    if std::env::var("CARGO_FEATURE_EMBED_ASSETS").is_ok() {
        let pkg = std::path::Path::new("../frontend/pkg");
        if !pkg.exists() || pkg.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            panic!(
                "\n\nembedding assets requires frontend/pkg/ to exist and be non-empty.\n\
                 Run this first:\n\n  \
                 wasm-pack build frontend/ --target web --release\n\n"
            );
        }
    }
}
```

If you forget to build the frontend first, you'll see a clear error message.

## Deployment

The resulting binary is completely self-contained:

```bash
# Copy to server
scp target/release/my-app-backend user@server:/path/to/

# Run directly (no external files needed)
./my-app-backend
```

### System Requirements

The target system only needs:

- A compatible operating system (Linux, macOS, Windows)
- No Rust toolchain required
- No wasm-pack required
- No external files

### Port Configuration

By default, the server runs on port 8080. Configure via environment variable or modify `BuildConfig`:

```rust
BuildConfig {
    port: std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080),
    // ...
}
```

## Build Size Optimization

### Strip Symbols

```bash
cargo build --release --features embed-assets
strip target/release/my-app-backend
```

### UPX Compression (Optional)

For further size reduction:

```bash
upx --best target/release/my-app-backend
```

### Profile Settings

Add to `Cargo.toml` for optimized release builds:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
panic = "abort"     # Smaller panic handling
strip = true        # Strip symbols
```

## Docker Deployment

```dockerfile
# Build stage
FROM rust:1.75 AS builder

RUN cargo install wasm-pack
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY . .

RUN wasm-pack build frontend/ --target web --release
RUN cargo build --release --features embed-assets

# Runtime stage
FROM debian:bookworm-slim

COPY --from=builder /app/target/release/my-app-backend /usr/local/bin/

EXPOSE 8080
CMD ["my-app-backend"]
```

## Comparison: Development vs Production

| Aspect | Development | Production |
|--------|-------------|------------|
| Build command | `cargo backend` | Two-step process |
| WASM optimization | None (`--dev`) | Full (`--release`) |
| Asset location | Filesystem | Embedded in binary |
| Live reload | Active | Disabled |
| Binary size | N/A | ~10-20 MB typical |
| Startup time | Slower (build first) | Instant |

## Troubleshooting

### "frontend/pkg/ is empty" Error

Run the frontend build first:

```bash
wasm-pack build frontend/ --target web --release
```

### Large Binary Size

1. Ensure release profile optimizations are enabled
2. Run `strip` on the binary
3. Consider UPX compression
4. Review dependencies for size impact

### Missing Assets at Runtime

This shouldn't happen with embedded assets. If it does:

1. Verify `embed-assets` feature is actually enabled
2. Check that `frontend/pkg/` was populated before backend build
3. Rebuild from clean state
