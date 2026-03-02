# Stage 6: Production Embedding

Stage 6 implements compile-time asset embedding for single-binary deployment.

## Overview

**Key Behavior:**
- `embed-assets` feature flag triggers embedding
- `build.rs` validates assets exist before compilation
- Assets embedded via `include_dir` and `include_bytes!`
- Dev-mode code compiled out

## Architecture

### Build Sequence

```bash
# Step 1: Build frontend
wasm-pack build frontend/ --target web --release

# Step 2: Build backend with embedded assets
cargo build --release --features embed-assets
```

### Compile-Time Behavior

| Feature | Without `embed-assets` | With `embed-assets` |
|---------|------------------------|---------------------|
| File watcher | Active | Compiled out |
| WebSocket endpoint | Available | Compiled out |
| Reload script | Injected | Not injected |
| Asset serving | From filesystem | From embedded bytes |

## Implementation Components

### Feature Flag

```toml
# backend/Cargo.toml
[features]
embed-assets = ["dep:include_dir"]

[dependencies.include_dir]
version = "0.7"
optional = true
```

### Build Guard

```rust
// backend/build.rs
fn main() {
    // Use env var, not cfg!(), for feature detection in build.rs
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

**Important:** Use `std::env::var("CARGO_FEATURE_EMBED_ASSETS")`, not `cfg!()`. The `cfg!()` macro does not work for feature detection in `build.rs`.

### Embedded Assets

```rust
// backend/src/build_subsystem/static_assets.rs

#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

#[cfg(feature = "embed-assets")]
static EMBEDDED_INDEX: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/index.html"));
```

### Conditional Handlers

```rust
// Dev mode implementation
#[cfg(not(feature = "embed-assets"))]
pub async fn serve_pkg_file(...) -> impl Responder {
    // Read from filesystem
}

// Release mode implementation
#[cfg(feature = "embed-assets")]
pub async fn serve_pkg_file(...) -> impl Responder {
    // Serve from EMBEDDED_PKG
}
```

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
- Compatible operating system
- No Rust toolchain required
- No wasm-pack required
- No external files

## Size Optimization

### Release Profile

```toml
# Cargo.toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
panic = "abort"     # Smaller panic handling
strip = true        # Strip symbols
```

### Strip Symbols

```bash
strip target/release/my-app-backend
```

### UPX Compression (Optional)

```bash
upx --best target/release/my-app-backend
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

## Test Coverage

- Embedded pkg is non-empty
- Embedded pkg contains WASM file
- Embedded index is non-empty
- Embedded index does not contain reload script

## Troubleshooting

### "frontend/pkg/ is empty" Error

Run the frontend build first:

```bash
wasm-pack build frontend/ --target web --release
```

### Large Binary Size

1. Ensure release profile optimizations
2. Run `strip` on the binary
3. Consider UPX compression
4. Review dependencies for size impact

### Missing Assets at Runtime

1. Verify `embed-assets` feature is enabled
2. Check `frontend/pkg/` was populated before build
3. Rebuild from clean state
