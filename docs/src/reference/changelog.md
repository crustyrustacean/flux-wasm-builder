# Changelog

All notable changes to flux-wasm-builder are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.2] - 2026-03-02

### Fixed

- Removed unused re-exports from scaffolded backend's `build_subsystem/mod.rs`
- Removed unused `reload_ws_path` field from `BuildConfig` in scaffolded projects
- Added `#[allow(dead_code)]` to `run_command_with_timeout` utility function
- Fixed clippy `single_match` and `redundant_closure` lints in build coordinator template
- Scaffolded backend now compiles without clippy warnings

## [0.6.0] - 2026-03-02

### Added

- Complete implementation of all 6 development stages
- Project scaffolding via `init` command
- wasm-pack build pipeline with timeout handling
- File watching with debouncing for `.rs` files
- WebSocket-based live reload for development
- Static asset serving with path traversal protection
- Shared types crate for API contracts
- Production asset embedding via `embed-assets` feature
- Comprehensive test suite (unit and integration tests)
- Structured logging via `tracing`
- mdbook documentation framework

### Changed

- Initial release with full feature set

### Security

- Path traversal protection in static asset handler
- Correct MIME types for WASM files (`application/wasm`)

## [0.5.0] - 2026-03-01

### Added

- Stage 5: Shared types crate implementation
- Compile-time type assertions for serialization
- Round-trip tests for shared types

## [0.4.0] - 2026-02-28

### Added

- Stage 4: Static asset serving
- SPA fallback for client-side routing
- MIME type detection via `mime_guess`

### Security

- Path traversal protection in `serve_pkg_file`

## [0.3.0] - 2026-02-27

### Added

- Stage 3: Live reload functionality
- WebSocket endpoint at `/ws/reload`
- Script injection into `index.html`
- Broadcast channel for multi-tab support

## [0.2.0] - 2026-02-26

### Added

- Stage 2: File watching
- `notify-debouncer-mini` integration
- Event filtering for `.rs` files
- Channel-based signaling to build coordinator

## [0.1.0] - 2026-02-25

### Added

- Stage 0: Project scaffolding
- `init` command with project name argument
- Environment validation (wasm-pack, wasm32 target)
- Template-based project generation

- Stage 1: Build pipeline
- wasm-pack subprocess invocation
- Timeout handling with process termination
- `BuildConfig` and `BuildError` types

### Dependencies

- clap 4.x for CLI parsing
- tokio 1.x for async runtime
- thiserror for error types
- tracing for logging
- actix-web 4.x for HTTP server
- actix-ws 0.4.x for WebSocket
- notify-debouncer-mini 0.7.x for file watching

---

## Version History Summary

| Version | Date | Description |
|---------|------|-------------|
| 0.7.2 | 2026-03-02 | Clippy lint fixes for scaffolded backend |
| 0.6.0 | 2026-03-02 | Complete implementation, documentation |
| 0.5.0 | 2026-03-01 | Shared types crate |
| 0.4.0 | 2026-02-28 | Static asset serving |
| 0.3.0 | 2026-02-27 | Live reload |
| 0.2.0 | 2026-02-26 | File watching |
| 0.1.0 | 2026-02-25 | Scaffolding, build pipeline |

---

## Future Considerations

Potential future enhancements (not committed):

- [ ] Custom port configuration via CLI flag
- [ ] Multiple frontend crate support
- [ ] CSS preprocessing (SASS/SCSS)
- [ ] Custom template support
- [ ] Build caching for faster rebuilds
- [ ] Hot module replacement (HMR)
- [ ] TypeScript declaration generation
- [ ] API client generation from shared types
