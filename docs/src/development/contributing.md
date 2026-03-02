# Contributing

Thank you for your interest in contributing to flux-wasm-builder!

## Development Setup

### Prerequisites

- Rust (edition 2024 compatible)
- wasm-pack >= 0.13.0
- wasm32-unknown-unknown target

```bash
# Install prerequisites
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

### Clone and Build

```bash
git clone https://github.com/crustyrustacean/flux-wasm-builder.git
cd flux-wasm-builder
cargo build
```

### Run Tests

```bash
# Standard tests
cargo test

# Tests requiring wasm-pack
RUN_WASM_TESTS=1 cargo test
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/my-feature
```

### 2. Make Changes

Follow the existing code style and patterns.

### 3. Write Tests

All new functionality must have tests:

- Unit tests in the module file (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directory

### 4. Run Tests

```bash
cargo test
cargo clippy
cargo fmt --check
```

### 5. Submit Pull Request

- Describe the change and motivation
- Reference any related issues
- Ensure CI passes

## Code Style

### Formatting

Use `cargo fmt`:

```bash
cargo fmt
```

### Linting

Use `cargo clippy`:

```bash
cargo clippy -- -D warnings
```

### Documentation

- All public items must have doc comments
- Include examples in doc comments where applicable

```rust
/// Run wasm-pack build with the given configuration.
///
/// # Arguments
///
/// * `config` - Build configuration
///
/// # Returns
///
/// `Ok(())` on success, `Err(BuildError)` on failure.
///
/// # Example
///
/// ```ignore
/// let config = BuildConfig::new("../frontend");
/// run_wasm_pack(&config).await?;
/// ```
pub async fn run_wasm_pack(config: &BuildConfig) -> Result<(), BuildError> {
    // ...
}
```

## Testing Guidelines

### Test Organization

| Test Type | Location |
|-----------|----------|
| Unit tests | `#[cfg(test)] mod tests` in source files |
| Integration tests | `tests/*.rs` |
| Documentation tests | In doc comments |

### Test Naming

```rust
#[test]
fn <subject>_<condition>_<expected_result>() {
    // ...
}

// Examples
#[test]
fn wasm_pack_returns_error_on_invalid_path() { }

#[test]
fn watcher_ignores_non_rs_files() { }
```

### Test Structure

Follow AAA pattern:

```rust
#[test]
fn test_name() {
    // Arrange
    let input = "test";
    
    // Act
    let result = function_under_test(input);
    
    // Assert
    assert_eq!(result, expected);
}
```

## Project Structure

```
flux-wasm-builder/
├── src/
│   ├── bin/main.rs        # CLI entry point
│   ├── lib.rs             # Library exports
│   ├── env_check.rs       # Environment validation
│   ├── init/              # Project scaffolding
│   └── build/             # Build subsystem
├── tests/                 # Integration tests
├── docs/                  # mdbook documentation
└── plans/                 # Implementation stage plans
```

## Pull Request Process

1. **Fork** the repository
2. **Create** a feature branch
3. **Write** tests first (TDD)
4. **Implement** the feature
5. **Document** public APIs
6. **Run** all tests and lints
7. **Submit** pull request

### PR Checklist

- [ ] Tests pass: `cargo test`
- [ ] Lints pass: `cargo clippy`
- [ ] Formatted: `cargo fmt`
- [ ] Documented: Public items have doc comments
- [ ] Changelog: Update if user-facing change

## Reporting Issues

When reporting issues, include:

1. **Environment**: OS, Rust version, wasm-pack version
2. **Steps to reproduce**: Minimal example
3. **Expected behavior**: What should happen
4. **Actual behavior**: What happened instead
5. **Logs**: Relevant output with `RUST_LOG=debug`

## Feature Requests

Feature requests are welcome! Please:

1. Check existing issues first
2. Describe the use case
3. Explain the expected behavior
4. Consider implementation approach

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
