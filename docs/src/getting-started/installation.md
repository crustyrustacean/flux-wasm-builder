# Installation

## From Source

Clone the repository and install locally:

```bash
git clone https://github.com/crustyrustacean/flux-wasm-builder.git
cd flux-wasm-builder
cargo install --path .
```

## Verify Installation

After installation, verify the tool is available:

```bash
flux-wasm-builder --help
```

Expected output:

```
flux-wasm-builder 

Commands:
  init  Create a new fullstack project
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Building from Source

If you want to contribute or modify the tool:

```bash
git clone https://github.com/crustyrustacean/flux-wasm-builder.git
cd flux-wasm-builder
cargo build
```

For development with all tests:

```bash
cargo test
```

For tests that require wasm-pack:

```bash
RUN_WASM_TESTS=1 cargo test
```
