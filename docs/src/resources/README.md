# Resources

A curated list of resources for the technologies wasm-drydock is built on and around.

## WebAssembly

- [WebAssembly Official Site](https://webassembly.org) — specification, concepts, and use cases
- [Rust and WebAssembly Book](https://rustwasm.github.io/docs/book/) — the definitive guide to compiling Rust to WASM
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) — the tool wasm-drydock uses to build your frontend
- [wasm-bindgen](https://rustwasm.github.io/docs/wasm-bindgen/) — how Rust and JavaScript interoperate at the WASM boundary

## Yew

- [Yew Documentation](https://yew.rs/docs/getting-started/introduction) — components, hooks, routing, and more
- [Yew 0.22 Release Notes](https://yew.rs/blog/2025/11/29/release-0-22) — what changed in the version wasm-drydock targets
- [Yew GitHub](https://github.com/yewstack/yew)
- [gloo](https://github.com/rustwasm/gloo) — toolkit for building WASM applications, used by Yew internally

## Actix-web

- [Actix-web Documentation](https://actix.rs/docs/) — handlers, middleware, extractors
- [Actix-web API Reference](https://docs.rs/actix-web) — full API docs on docs.rs

## Trunk

- [Trunk](https://trunkrs.dev) — the WASM bundler wasm-drydock does not use but which you may encounter in the Yew ecosystem. Understanding trunk helps clarify what wasm-drydock replaces.

## Rust Async

- [Tokio Documentation](https://tokio.rs) — the async runtime wasm-drydock is built on
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) — recommended reading for understanding the async foundations

## Deployment

- [Fly.io Documentation](https://fly.io/docs/) — the deployment platform covered in this guide
- [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) — Docker layer caching for Rust projects, used in the wasm-drydock Dockerfile
- [Caddy](https://caddyserver.com) — a simple reverse proxy with automatic HTTPS, a good alternative to Fly for VPS deployments

## Related Projects

- [Leptos](https://leptos.dev) — a full-stack Rust framework with SSR support, worth knowing about as the ecosystem matures
- [Dioxus](https://dioxuslabs.com) — another Rust UI framework targeting multiple platforms including WASM
- [cargo-generate](https://github.com/cargo-generate/cargo-generate) — template-based project scaffolding, an alternative approach to what wasm-drydock's `init` command does
