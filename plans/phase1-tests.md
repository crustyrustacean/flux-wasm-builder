# Phase 1 Test Implementation Plan

## Overview

This plan covers implementing stable tests that won't require changes when the web server and reverse proxy are added. These tests focus on the domain configuration module and CLI command existence.

## Files to Create/Modify

### 1. Add Unit Tests to `src/domain.rs`

**File:** `src/domain.rs`
**Location:** Add `#[cfg(test)]` module at the end of the file

**Tests to implement:**

| Test Name | Description |
|-----------|-------------|
| `dev_config_defaults` | Verify `DevConfig::default()` returns expected values |
| `flux_config_parses_valid_toml` | Parse a complete flux.toml with all fields |
| `flux_config_uses_dev_defaults` | Parse minimal flux.toml, verify defaults applied |
| `flux_config_missing_file_returns_error` | Verify `ConfigError::ReadFailed` for missing file |
| `flux_config_invalid_toml_returns_error` | Verify `ConfigError::ParseFailed` for invalid TOML |
| `flux_config_missing_project_name_returns_error` | Verify error when `[project]` section missing `name` |

**Implementation notes:**
- Use `tempfile::tempdir()` for creating temporary directories
- Use `std::fs::File` and `writeln!` for creating test config files
- Match on specific error variants using `matches!` macro

### 2. Create Integration Test File `tests/domain.rs`

**File:** `tests/domain.rs` (new file)

**Tests to implement:**

| Test Name | Description |
|-----------|-------------|
| `flux_config_parses_complete_config` | Full config with custom dev settings |
| `flux_config_missing_dev_section_uses_defaults` | Minimal config, verify defaults |

**Implementation notes:**
- Import `FluxConfig` and `ConfigError` from `flux_wasm_builder`
- Use `tempfile::tempdir()` for isolation
- Test the public API only

### 3. Add Test to `tests/init.rs`

**File:** `tests/init.rs`
**Location:** Add at the end of the file

**Test to add:**

| Test Name | Description |
|-----------|-------------|
| `dev_command_exists` | Verify `dev` subcommand exists and fails gracefully without flux.toml |

**Implementation notes:**
- Use `cargo_bin_cmd!("flux-wasm-builder")` with `args(["dev"])`
- Run in empty temp directory
- Assert `.failure()` (expected since no flux.toml exists)

## Execution Order

1. Add unit tests to `src/domain.rs` first
2. Run `cargo test --lib domain` to verify
3. Create `tests/domain.rs` integration tests
4. Run `cargo test --test domain` to verify
5. Add test to `tests/init.rs`
6. Run `cargo test --test init` to verify
7. Run full test suite: `cargo test`

## Expected Test Output

```
running 6 tests
test domain::tests::dev_config_defaults ... ok
test domain::tests::flux_config_parses_valid_toml ... ok
test domain::tests::flux_config_uses_dev_defaults ... ok
test domain::tests::flux_config_missing_file_returns_error ... ok
test domain::tests::flux_config_invalid_toml_returns_error ... ok
test domain::tests::flux_config_missing_project_name_returns_error ... ok

test result: ok. 6 passed; 0 failed; 0 ignored
```

## Dependencies

No new dependencies needed. The existing `tempfile` dev-dependency in `Cargo.toml` is sufficient.

## Verification Checklist

- [ ] All 6 unit tests in `src/domain.rs` pass
- [ ] All 2 integration tests in `tests/domain.rs` pass
- [ ] New test in `tests/init.rs` passes
- [ ] Existing tests still pass: `cargo test`
- [ ] No compiler warnings: `cargo check`
