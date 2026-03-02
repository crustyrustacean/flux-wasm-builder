# Static Assets

The static asset serving module handles WASM, JavaScript, and other build artifacts.

## Module

| File | Purpose |
|------|---------|
| [`static_assets.rs`](https://github.com/crustyrustacean/flux-wasm-builder/blob/main/src/build/static_assets.rs) | Asset serving handlers |

## Handlers

### serve_pkg_file

Serves files from the `pkg/` directory:

```rust
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder
```

**Dev Mode:** Reads from filesystem at runtime
**Release Mode:** Serves from embedded bytes

### spa_fallback

Returns `index.html` for unmatched routes:

```rust
pub async fn spa_fallback(
    config: web::Data<BuildConfig>,
    dev_mode: web::Data<DevMode>,
) -> impl Responder
```

Enables client-side routing in Yew applications.

## Security: Path Traversal Protection

The handler uses `file_name()` to extract only the final path component:

```rust
let safe_name = match std::path::Path::new(&filename).file_name() {
    Some(n) => n.to_owned(),
    None => {
        tracing::warn!("path traversal attempt blocked");
        return HttpResponse::BadRequest().body("invalid filename");
    }
};
```

This prevents attacks like `/pkg/../../etc/passwd` from escaping the pkg directory.

### Attack Example

Request: `GET /pkg/..%2F..%2Fetc%2Fpasswd`

1. URL decodes to `/pkg/../../etc/passwd`
2. `file_name()` extracts just `passwd`
3. Server looks for `pkg/passwd` (not found)
4. Returns 404, not `/etc/passwd`

## MIME Types

MIME types are detected via `mime_guess`:

```rust
let mime = mime_guess::from_path(&safe_name).first_or_octet_stream();
```

### Common Types

| Extension | MIME Type |
|-----------|-----------|
| `.wasm` | `application/wasm` |
| `.js` | `application/javascript` |
| `.html` | `text/html` |
| `.css` | `text/css` |
| `.json` | `application/json` |
| Unknown | `application/octet-stream` |

### WASM Streaming

The `application/wasm` MIME type enables browser streaming instantiation:

```javascript
WebAssembly.instantiateStreaming(fetch('/pkg/app_bg.wasm'), imports);
```

## Dev Mode Implementation

```rust
#[cfg(not(feature = "embed-assets"))]
pub async fn serve_pkg_file(
    path: web::Path<String>,
    config: web::Data<BuildConfig>,
) -> impl Responder {
    let file_path = config.pkg_output_path.join(&safe_name);
    
    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(bytes)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::NotFound().finish()
        }
        Err(e) => {
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

## Release Mode Implementation

```rust
#[cfg(feature = "embed-assets")]
static EMBEDDED_PKG: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../frontend/pkg");

pub async fn serve_pkg_file(
    path: web::Path<String>,
    _config: web::Data<BuildConfig>,
) -> impl Responder {
    match EMBEDDED_PKG.get_file(&safe_name) {
        Some(file) => {
            HttpResponse::Ok()
                .content_type(mime.to_string())
                .body(file.contents().to_vec())
        }
        None => {
            HttpResponse::NotFound().finish()
        }
    }
}
```

## SPA Fallback

### Route Registration

```rust
App::new()
    .configure(api::configure)           // API routes first
    .route("/pkg/{filename}", web::get().to(serve_pkg_file))
    .default_service(web::get().to(spa_fallback))  // Last
```

The `default_service` ensures it cannot shadow `/pkg/` or `/api/` paths.

### Dev Mode: Script Injection

```rust
let html = if dev_mode.0 {
    let html_str = String::from_utf8_lossy(&bytes);
    inject_reload_script(&html_str).into_bytes()
} else {
    bytes
};
```

### Release Mode: Embedded HTML

```rust
#[cfg(feature = "embed-assets")]
static EMBEDDED_INDEX: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/index.html"));
```

## Route Priority

| Priority | Route | Handler |
|----------|-------|---------|
| 1 | `/api/*` | Application handlers |
| 2 | `/pkg/{filename}` | `serve_pkg_file` |
| 3 | `*` (default) | `spa_fallback` |

This order ensures:
- API routes always match first
- Static assets are served correctly
- All other routes return `index.html` for client-side routing

## Testing

```rust
#[actix_web::test]
async fn serves_wasm_with_application_wasm_content_type() {
    let pkg_dir = tempdir().unwrap();
    std::fs::write(pkg_dir.path().join("app_bg.wasm"), b"\0asm").unwrap();
    
    let config = BuildConfig {
        pkg_output_path: pkg_dir.path().to_path_buf(),
        ..BuildConfig::new("/unused")
    };
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(config))
            .route("/pkg/{filename}", web::get().to(serve_pkg_file))
    ).await;
    
    let req = test::TestRequest::get().uri("/pkg/app_bg.wasm").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), 200);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("application/wasm"));
}

#[actix_web::test]
async fn path_traversal_attempt_is_rejected() {
    // Request: /pkg/..%2Fsecret.txt
    let req = test::TestRequest::get()
        .uri("/pkg/..%2Fsecret.txt")
        .to_request();
    
    let status = test::call_service(&app, req).await.status();
    assert_ne!(status.as_u16(), 200, "path traversal must not return 200");
}
```

## Caching

For production deployments, consider adding cache headers:

```rust
HttpResponse::Ok()
    .content_type(mime.to_string())
    .insert_header(("Cache-Control", "public, max-age=31536000"))  // 1 year
    .body(bytes)
```

Note: This requires modifying the generated code in your project.
