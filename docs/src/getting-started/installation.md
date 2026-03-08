# Installation

## Prerequisites

Before installing wasm-drydock, ensure the following are available on your system.

**Rust toolchain (edition 2024)**

Install via [rustup](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**wasm-pack**

wasm-drydock uses wasm-pack to compile your Yew frontend to WebAssembly.

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Minimum required version: `0.13.0`. Verify with:

```bash
wasm-pack --version
```

**wasm32-unknown-unknown target**

```bash
rustup target add wasm32-unknown-unknown
```

## Installing wasm-drydock

```bash
cargo install wasm-drydock
```

Verify the installation:

```bash
wasm-drydock --help
```

## Updating

```bash
cargo install wasm-drydock --force
```
