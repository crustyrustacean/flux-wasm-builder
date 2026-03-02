# Prerequisites

Before using flux-wasm-builder, ensure you have the following tools installed.

## Required Tools

### Rust

flux-wasm-builder requires Rust with edition 2024 support.

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### wasm-pack

wasm-pack is required for building WebAssembly from Rust. Version 0.13.0 or higher is required.

```bash
# Install wasm-pack
cargo install wasm-pack

# Verify installation
wasm-pack --version
```

If wasm-pack is not found or the version is too old, the `init` command will display an error with installation instructions.

### wasm32-unknown-unknown Target

The WebAssembly target must be installed via rustup:

```bash
# Install the target
rustup target add wasm32-unknown-unknown

# Verify installation
rustup target list --installed | grep wasm32
```

## Environment Validation

The `init` command automatically validates your environment:

```bash
flux-wasm-builder init my-app
```

If prerequisites are missing, you'll see actionable error messages:

```
error: wasm-pack not found on PATH — install from https://rustwasm.github.io/wasm-pack/
```

```
error: wasm-pack version 0.12.0 is below minimum required 0.13.0
```

```
error: wasm32-unknown-unknown target not installed — run: rustup target add wasm32-unknown-unknown
```

## Optional Tools

### mdbook

For building the documentation locally:

```bash
cargo install mdbook
```

### git

For version control (recommended):

```bash
git --version
```

## System Requirements

- **Operating System**: Linux, macOS, or Windows
- **Memory**: At least 4GB RAM recommended for WASM builds
- **Disk Space**: ~2GB for Rust toolchain and dependencies

## Troubleshooting

### wasm-pack Installation Fails

On some systems, you may need additional dependencies:

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install build-essential curl
```

**macOS:**
```bash
xcode-select --install
```

### Slow First Build

The first `wasm-pack build` will be slow as it downloads and compiles dependencies. Subsequent builds are significantly faster due to caching.

### Port Already in Use

If port 8080 is already in use:

```
error: port 8080 is already in use. Change the port in BuildConfig.
```

Modify the port in `backend/src/main.rs` or stop the conflicting process.
