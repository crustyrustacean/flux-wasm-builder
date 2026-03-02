# Stage 5: Shared Types

Stage 5 implements the shared types crate for API contract enforcement.

## Overview

**Key Behavior:**
- Define API types in one place
- Compile for both WASM and native targets
- Serde serialization for HTTP boundary
- Compile-time type assertions

## Architecture

```
shared/
├── Cargo.toml
└── src/
    └── lib.rs        # API types

backend/               frontend/
    │                      │
    └──────┬───────────────┘
           │
           ▼
       shared/    (single source of truth)
```

## Constraints

The shared crate must:

1. **Depend only on `serde`** with `derive` feature
2. **Compile for both targets**:
   - `wasm32-unknown-unknown` (frontend)
   - Host target (backend)
3. **Never pull in**:
   - `web-sys`
   - `wasm-bindgen`
   - `actix-web`
   - Any crate that doesn't support WASM

## Implementation Components

### Generated Types

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

### Compile-Time Assertions

```rust
const _: fn() = || {
    fn assert_serialize<T: serde::Serialize>() {}
    fn assert_deserialize<T: serde::de::DeserializeOwned>() {}
    
    assert_serialize::<HelloResponse>();
    assert_deserialize::<HelloResponse>();
    assert_serialize::<StatusResponse>();
    assert_deserialize::<StatusResponse>();
};
```

This catches serialization issues at compile time.

### Round-Trip Tests

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

## Important: No deny_unknown_fields

**Never use `#[serde(deny_unknown_fields)]` on shared types.**

This is a forward-compatibility decision:

```rust
// WRONG - will break old frontends
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserResponse {
    pub name: String,
}

// CORRECT - old frontends ignore new fields
#[derive(Serialize, Deserialize)]
pub struct UserResponse {
    pub name: String,
    pub email: Option<String>,  // Adding won't break old frontends
}
```

## Backend Usage

```rust
// backend/src/api/mod.rs
use my_app_shared::{HelloResponse, StatusResponse};

pub async fn hello() -> impl actix_web::Responder {
    HttpResponse::Ok().json(HelloResponse {
        message: "Hello from Actix-web!".to_string(),
    })
}
```

## Frontend Usage

```rust
// frontend/src/lib.rs
use my_app_shared::StatusResponse;
use gloo_net::http::Request;

async fn fetch_status() -> StatusResponse {
    Request::get("/api/status")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}
```

## Cargo.toml

```toml
[package]
name = "my-app-shared"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
serde_json = "1"
```

Note: `serde_json` is only a dev-dependency for tests.

## Test Coverage

- API mod imports StatusResponse
- API mod imports HelloResponse
- Status handler uses StatusResponse struct
- Status handler uses CARGO_PKG_VERSION
- Shared lib has compile-time assertions
- Shared lib has round-trip tests
- Shared Cargo.toml has serde_json dev-dependency
- Frontend fetches /api/status
- API responses use JSON content type
