# Project Structure

A wasm-drydock project is a three-crate Cargo workspace. Each crate has a clearly defined responsibility:

| Crate | Purpose |
|-------|---------|
| `backend/` | Actix-web API server |
| `frontend/` | Yew WASM application |
| `shared/` | Serde-compatible API types |

The `shared` crate is the boundary between frontend and backend. Both crates depend on it, so serialization mismatches become compile errors rather than runtime surprises.
