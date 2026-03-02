# Stage 4: Static Asset Serving

Stage 4 implements static asset serving for WASM, JavaScript, and SPA fallback.

## Overview

**Key Behavior:**
- Serve files from `pkg/` directory
- Correct MIME types (especially `application/wasm`)
- SPA fallback for client-side routing
- Path traversal protection

## Architecture

### Request Routing

| Priority | Route | Handler |
|----------|-------|---------|
| 1 | `/api/*` | Application handlers |
| 2 | `/pkg/{filename}` | `serve_pkg_file` |
| 3 | `*` (default) | `spa_fallback` |

## Implementation Components

### serve_pkg_file

```rust
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder
```

Serves files from `pkg/` directory with:
- Path traversal protection
- MIME type detection
- Dev mode: filesystem read
- Release mode: embedded bytes

### spa_fallback

```rust
pub async fn spa_fallback(
    config: web::Data<BuildConfig>,
    dev_mode: web::Data<DevMode>,
) -> impl Responder
```

Returns `index.html` for unmatched routes:
- Dev mode: Inject reload script
- Release mode: Serve as-is

## Security: Path Traversal Protection

```rust
let safe_name = match std::path::Path::new(&filename).file_name() {
    Some(n) => n.to_owned(),
    None => {
        tracing::warn!("path traversal attempt blocked");
        return HttpResponse::BadRequest().body("invalid filename");
    }
};
```

This prevents attacks like `/pkg/../../etc/passwd`:
- `file_name()` extracts only the final component
- Request for `../../etc/passwd` becomes just `passwd`
- Server looks for `pkg/passwd` (not found)

## MIME Types

```rust
let mime = mime_guess::from_path(&safe_name).first_or_octet_stream();
```

| Extension | MIME Type |
|-----------|-----------|
| `.wasm` | `application/wasm` |
| `.js` | `application/javascript` |
| `.html` | `text/html` |
| `.css` | `text/css` |
| Unknown | `application/octet-stream` |

### WASM Streaming

The `application/wasm` MIME type enables browser streaming instantiation:

```javascript
WebAssembly.instantiateStreaming(fetch('/pkg/app_bg.wasm'), imports);
```

## Dev Mode Implementation

```rust
#[cfg(not(feature = "embed-assets"))]
pub async fn serve_pkg_file(...) -> impl Responder {
    let file_path = config.pkg_output_path.join(&safe_name);
    match tokio::fs::read(&file_path).await {
        Ok(bytes) => HttpResponse::Ok()
            .content_type(mime.to_string())
            .body(bytes),
        Err(e) if e.kind() == NotFound => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
```

## Release Mode Implementation

```rust
#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

pub async fn serve_pkg_file(...) -> impl Responder {
    match EMBEDDED_PKG.get_file(&safe_name) {
        Some(file) => HttpResponse::Ok()
            .content_type(mime.to_string())
            .body(file.contents().to_vec()),
        None => HttpResponse::NotFound().finish(),
    }
}
```

## Test Coverage

- Serves JS file from pkg directory
- Serves WASM with `application/wasm` content type
- Returns 404 for missing file
- Path traversal attempt is rejected
- SPA fallback serves index.html
- API route takes precedence over SPA fallback
- Index.html contains reload script when dev mode true
- Index.html omits reload script when dev mode false

## Route Registration

```rust
App::new()
    .configure(api::configure)           // API routes first
    .route("/pkg/{filename}", web::get().to(serve_pkg_file))
    .default_service(web::get().to(spa_fallback))  // Last
```

The `default_service` ensures it cannot shadow `/pkg/` or `/api/` paths.
