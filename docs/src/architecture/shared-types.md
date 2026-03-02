# Shared Types

The `shared` crate enforces API contracts at the type system level.

## Purpose

Any struct or enum that crosses the HTTP boundary should be defined in the `shared` crate. Both backend and frontend depend on this crate, ensuring:

- Serialization mismatches are compile errors
- API contracts are explicit in code
- Types are documented in one place

## Constraints

The shared crate must:

1. **Depend only on `serde`** with the `derive` feature
2. **Compile for both targets**:
   - `wasm32-unknown-unknown` (frontend)
   - Host target (backend)
3. **Never pull in**:
   - `web-sys`
   - `wasm-bindgen`
   - `actix-web`
   - Any crate that doesn't support WASM

## Generated Types

```rust
// shared/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
}
```

## Important: No deny_unknown_fields

**Never use `#[serde(deny_unknown_fields)]` on shared types.**

This is a forward-compatibility decision:

```rust
// WRONG - will break old frontends when backend adds fields
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserResponse {
    pub name: String,
}

// CORRECT - old frontends ignore new fields
#[derive(Serialize, Deserialize)]
pub struct UserResponse {
    pub name: String,
    // Adding this field won't break old frontends
    pub email: Option<String>,
}
```

When the backend adds new fields, older compiled frontends will safely ignore them rather than panicking at runtime.

## Backend Usage

```rust
// backend/src/api/mod.rs
use my_app_shared::{HelloResponse, StatusResponse};

pub async fn hello() -> impl actix_web::Responder {
    HttpResponse::Ok().json(HelloResponse {
        message: "Hello from Actix-web!".to_string(),
    })
}

pub async fn status() -> impl actix_web::Responder {
    HttpResponse::Ok().json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime(),
    })
}
```

## Frontend Usage

```rust
// frontend/src/lib.rs
use my_app_shared::StatusResponse;
use gloo_net::http::Request;

async fn fetch_status() -> StatusResponse {
    let resp = Request::get("/api/status")
        .send()
        .await
        .unwrap();
    resp.json()
        .await
        .unwrap()
}
```

## Compile-Time Assertions

The generated `shared/src/lib.rs` includes compile-time type assertions:

```rust
// Compile-time assertion that types implement Serialize/Deserialize
const _: fn() = || {
    fn assert_serialize<T: serde::Serialize>() {}
    fn assert_deserialize<T: serde::de::DeserializeOwned>() {}
    
    assert_serialize::<HelloResponse>();
    assert_deserialize::<HelloResponse>();
    assert_serialize::<StatusResponse>();
    assert_deserialize::<StatusResponse>();
};
```

This catches serialization issues at compile time, not runtime.

## Round-Trip Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn hello_response_round_trips_through_json() {
        let original = HelloResponse {
            message: "test".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: HelloResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(original.message, decoded.message);
    }
    
    #[test]
    fn unknown_fields_are_ignored_not_rejected() {
        let json = r#"{"message":"hi","extra":"ignored"}"#;
        let decoded: HelloResponse = serde_json::from_str(json).unwrap();
        assert_eq!(decoded.message, "hi");
    }
}
```

## Adding New Types

1. Define the type in `shared/src/lib.rs`
2. Add compile-time assertions
3. Add round-trip tests
4. Use in both backend and frontend

```rust
// shared/src/lib.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: u64,
    pub username: String,
    pub display_name: String,
}

// Add assertions
const _: fn() = || {
    // ... existing assertions ...
    assert_serialize::<UserResponse>();
    assert_deserialize::<UserResponse>();
};
```

## Cargo.toml

```toml
# shared/Cargo.toml
[package]
name = "my-app-shared"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
serde_json = "1"
```

Note: `serde_json` is only a dev-dependency for tests, not a runtime dependency.

## Best Practices

1. **Keep types simple**: Avoid complex generics or lifetimes
2. **Use owned types**: `String` not `&str`, `Vec<T>` not `&[T]`
3. **Document fields**: Add doc comments for API documentation
4. **Version carefully**: Adding fields is safe; removing/renaming is breaking

```rust
/// Response from the /api/user/:id endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    /// Unique user identifier
    pub id: u64,
    /// Login username
    pub username: String,
    /// Display name shown in UI
    pub display_name: String,
}
```
