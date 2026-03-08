# Shared

The `shared` crate contains the API boundary types used by both `backend` and `frontend`.

Both crates depend on `shared`, so serialization mismatches between the API and the frontend are caught at compile time rather than at runtime.

## Types

```rust
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

pub struct HelloResponse {
    pub message: String,
}

pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
}
```

## Design notes

- Types do not use `#[serde(deny_unknown_fields)]` — unknown fields are silently ignored, which ensures forward compatibility as the API evolves
- Compile-time assertions verify that all types implement `Serialize` and `Deserialize`
