# Stage 6 — Shared Types Integration

**Status:** Planning
**Prerequisites:** Stages 0-5 (All Complete)

---

## 1. Executive Summary

Stage 6 establishes the shared types pattern as a working, tested foundation. It confirms the compile-time contract between frontend and backend, enforces the forward-compatibility stance (no `deny_unknown_fields`), and validates WASM target compatibility. It also completes the `/api/status` handler to use the `StatusResponse` shared type (currently it returns ad-hoc JSON) and updates the frontend to fetch and display both endpoints using shared types.

### Key Deliverables

1. **Updated `shared/src/lib.rs` template** — Add `serde_json` dev-dependency, unit tests for JSON round-tripping, unknown-field tolerance, and compile-time trait assertions
2. **Updated `backend/src/api/mod.rs` template** — Use `StatusResponse` from the shared crate instead of ad-hoc `serde_json::json!()`
3. **Updated `frontend/src/lib.rs` template** — Fetch `/api/status` on mount, display version string
4. **Updated `shared/Cargo.toml` template** — Add `serde_json` as dev-dependency for tests
5. **Integration tests** — New `tests/api.rs` in the tool crate verifying API handler behavior with shared types
6. **Scaffold tests** — Verify generated files contain the expected content

---

## 2. Current State Analysis

### What Exists

| Component | Location | Status |
|-----------|----------|--------|
| `shared/src/lib.rs` template | `src/init/templates.rs` (`shared_lib_rs()`) | Has `HelloResponse` + `StatusResponse` structs, no tests, no compile-time assertions |
| `shared/Cargo.toml` template | `src/init/templates.rs` (`shared_cargo_toml()`) | Has `serde` dependency, no `serde_json` dev-dependency |
| `backend/src/api/mod.rs` template | `src/init/templates.rs` (`backend_api_mod()`) | Has `/api/hello` using `HelloResponse`, has `/api/status` but uses ad-hoc `serde_json::json!()` instead of `StatusResponse` |
| `frontend/src/lib.rs` template | `src/init/templates.rs` (`frontend_lib_rs()`) | Fetches `/api/hello` only, does not fetch `/api/status` |
| `tests/api.rs` | Does not exist | No integration tests for API handlers |

### What Needs to Change

1. **`shared_lib_rs()` template** — Add unit tests (`hello_response_round_trips_through_json`, `status_response_round_trips_through_json`, `unknown_fields_are_ignored_not_rejected`) and the compile-time trait assertion block
2. **`shared_cargo_toml()` template** — Add `serde_json = "1"` as `[dev-dependencies]` for the unit tests
3. **`backend_api_mod()` template** — Import and use `StatusResponse` for the `/api/status` handler instead of `serde_json::json!()`
4. **`frontend_lib_rs()` template** — Import `StatusResponse`, fetch `/api/status` on mount, display the version
5. **`tests/api.rs`** — New integration test file validating API handlers return correct shared types with `application/json` content type
6. **Existing scaffold tests** — Add tests verifying shared types tests and compile-time assertions appear in generated code

---

## 3. Implementation Steps

### Step 3.1: Update `shared_cargo_toml()` Template

**File:** `src/init/templates.rs`

Add `serde_json` as a dev-dependency so the shared crate's unit tests can test JSON round-tripping:

```rust
pub fn shared_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}-shared"
version.workspace = true
edition.workspace = true

[dependencies]
serde = {{ version = "1", features = ["derive"] }}

[dev-dependencies]
serde_json = "1"
"#
    )
}
```

**Rationale:** The spec's unit tests in Section 8 Stage 6 call `serde_json::to_string` and `serde_json::from_str` on the shared types. These tests live inside the shared crate itself, so it needs `serde_json` as a dev-dependency.

---

### Step 3.2: Update `shared_lib_rs()` Template

**File:** `src/init/templates.rs`

Add unit tests and compile-time assertions per the spec:

```rust
pub fn shared_lib_rs() -> &'static str {
    r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
}

// Compile-time assertions: field renames or trait removals become
// compile errors rather than runtime failures.
const _: fn() = || {
    fn assert_serialize<T: serde::Serialize>() {}
    fn assert_deserialize<T: for<'de> serde::Deserialize<'de>>() {}
    assert_serialize::<HelloResponse>();
    assert_deserialize::<HelloResponse>();
    assert_serialize::<StatusResponse>();
    assert_deserialize::<StatusResponse>();
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_response_round_trips_through_json() {
        let r = HelloResponse { message: "hi".into() };
        let json = serde_json::to_string(&r).unwrap();
        let r2: HelloResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.message, "hi");
    }

    #[test]
    fn status_response_round_trips_through_json() {
        let r = StatusResponse { version: "0.1.0".into(), uptime_seconds: 42 };
        let json = serde_json::to_string(&r).unwrap();
        let r2: StatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.version, "0.1.0");
        assert_eq!(r2.uptime_seconds, 42);
    }

    #[test]
    fn unknown_fields_are_ignored_not_rejected() {
        // Verifies deny_unknown_fields is absent — see Section 5.2
        let json = r#"{"message":"hi","future_field":true}"#;
        let r: HelloResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.message, "hi");
    }
}
"#
}
```

**Key design decisions:**
- Tests are inline in the generated `shared` crate so they run with `cargo test` in the generated project
- The compile-time assertion block uses the `const _: fn() = || { ... }` pattern — it's evaluated at compile time and ensures `Serialize`/`Deserialize` impls exist without producing runtime code
- No `deny_unknown_fields` anywhere, enforced by the `unknown_fields_are_ignored_not_rejected` test

---

### Step 3.3: Update `backend_api_mod()` Template

**File:** `src/init/templates.rs`

Change the `/api/status` handler to use `StatusResponse` from the shared crate instead of ad-hoc JSON:

```rust
pub fn backend_api_mod(name: &str) -> String {
    let crate_name = name.replace('-', "_");
    format!(
        r#"use actix_web::{{web, HttpResponse, Responder}};
use {crate_name}_shared::{{HelloResponse, StatusResponse}};

pub fn configure(cfg: &mut web::ServiceConfig) {{
    cfg.service(
        web::scope("/api")
            .route("/hello", web::get().to(hello))
            .route("/status", web::get().to(status))
    );
}}

async fn hello() -> impl Responder {{
    HttpResponse::Ok().json(HelloResponse {{
        message: "Hello from the backend!".to_string(),
    }})
}}

async fn status() -> impl Responder {{
    HttpResponse::Ok().json(StatusResponse {{
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0,
    }})
}}
"#
    )
}
```

**Changes from current:**
1. Import `StatusResponse` alongside `HelloResponse` from the shared crate
2. Remove `use serde_json::json;` — no longer needed
3. Replace `json!({ "version": ..., "status": "ok" })` with `StatusResponse { version: ..., uptime_seconds: 0 }`

**Why `uptime_seconds: 0`:** The spec says `uptime_seconds: 0` for the initial implementation. A real uptime counter could be added later but is out of scope for this stage.

---

### Step 3.4: Update `frontend_lib_rs()` Template

**File:** `src/init/templates.rs`

Update the frontend to import `StatusResponse`, fetch `/api/status` on mount, and display both the hello message and the version string:

```rust
pub fn frontend_lib_rs(name: &str) -> String {
    let crate_name = name.replace('-', "_");
    format!(
        r#"use yew::prelude::*;
use gloo_net::http::Request;
use {crate_name}_shared::{{HelloResponse, StatusResponse}};

#[component(App)]
fn app() -> Html {{
    let message = use_state(|| None::<String>);
    let version = use_state(|| None::<String>);

    {{
        let message = message.clone();
        use_effect_with((), move |_| {{
            let message = message.clone();
            wasm_bindgen_futures::spawn_local(async move {{
                match Request::get("/api/hello")
                    .send()
                    .await
                {{
                    Ok(response) => {{
                        if let Ok(hello) = response.json::<HelloResponse>().await {{
                            message.set(Some(hello.message));
                        }}
                    }}
                    Err(e) => {{
                        web_sys::console::log_1(&format!("Error: {{:?}}", e).into());
                    }}
                }}
            }});
            || ()
        }});
    }}

    {{
        let version = version.clone();
        use_effect_with((), move |_| {{
            let version = version.clone();
            wasm_bindgen_futures::spawn_local(async move {{
                match Request::get("/api/status")
                    .send()
                    .await
                {{
                    Ok(response) => {{
                        if let Ok(status) = response.json::<StatusResponse>().await {{
                            version.set(Some(status.version));
                        }}
                    }}
                    Err(e) => {{
                        web_sys::console::log_1(&format!("Error: {{:?}}", e).into());
                    }}
                }}
            }});
            || ()
        }});
    }}

    html! {{
        <div>
            <h1>{{ "Flux WASM Builder" }}</h1>
            {{
                if let Some(msg) = (*message).clone() {{
                    html! {{ <p>{{ msg }}</p> }}
                }} else {{
                    html! {{ <p>{{ "Loading..." }}</p> }}
                }}
            }}
            {{
                if let Some(ver) = (*version).clone() {{
                    html! {{ <p class="version">{{ format!("v{{}}", ver) }}</p> }}
                }} else {{
                    html! {{ <></> }}
                }}
            }}
        </div>
    }}
}}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {{
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}}
"#
    )
}
```

**Changes from current:**
1. Import `StatusResponse` alongside `HelloResponse`
2. Add `version` state hook
3. Add second `use_effect_with((), ...)` block to fetch `/api/status`
4. Display version string in the HTML output

**Yew version note:** Uses `use_effect_with((), ...)` which is the Yew 0.21/0.22 hook signature. The spec says "Yew 0.21" but the current templates already use Yew 0.22.1 — we keep 0.22.1 as established.

---

### Step 3.5: Create Integration Tests

**File:** `tests/api.rs` (NEW)

These tests verify the generated API handlers work correctly with shared types:

```rust
use tempfile::tempdir;

#[test]
fn scaffolded_api_mod_imports_status_response() {
    let dir = tempdir().unwrap();

    assert_cmd::Command::cargo_bin("flux-wasm-builder")
        .unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let api_mod = std::fs::read_to_string(
        dir.path().join("test-app/backend/src/api/mod.rs"),
    )
    .unwrap();

    assert!(
        api_mod.contains("StatusResponse"),
        "api/mod.rs should import StatusResponse from shared crate"
    );
    assert!(
        api_mod.contains("HelloResponse"),
        "api/mod.rs should import HelloResponse from shared crate"
    );
    // Should NOT use serde_json::json! for status endpoint
    assert!(
        !api_mod.contains("json!"),
        "status handler should use StatusResponse, not json!()"
    );
}

#[test]
fn scaffolded_api_status_uses_shared_type() {
    let dir = tempdir().unwrap();

    assert_cmd::Command::cargo_bin("flux-wasm-builder")
        .unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let api_mod = std::fs::read_to_string(
        dir.path().join("test-app/backend/src/api/mod.rs"),
    )
    .unwrap();

    assert!(
        api_mod.contains("StatusResponse {"),
        "status handler should construct a StatusResponse struct"
    );
    assert!(
        api_mod.contains("CARGO_PKG_VERSION"),
        "status handler should use CARGO_PKG_VERSION for version"
    );
    assert!(
        api_mod.contains("uptime_seconds"),
        "status handler should include uptime_seconds field"
    );
}

#[test]
fn scaffolded_shared_lib_has_compile_time_assertions() {
    let dir = tempdir().unwrap();

    assert_cmd::Command::cargo_bin("flux-wasm-builder")
        .unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let shared_lib = std::fs::read_to_string(
        dir.path().join("test-app/shared/src/lib.rs"),
    )
    .unwrap();

    assert!(
        shared_lib.contains("assert_serialize"),
        "shared lib should have compile-time serialize assertion"
    );
    assert!(
        shared_lib.contains("assert_deserialize"),
        "shared lib should have compile-time deserialize assertion"
    );
    assert!(
        shared_lib.contains("const _: fn()"),
        "shared lib should use const fn pattern for compile-time checks"
    );
}

#[test]
fn scaffolded_shared_lib_has_round_trip_tests() {
    let dir = tempdir().unwrap();

    assert_cmd::Command::cargo_bin("flux-wasm-builder")
        .unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let shared_lib = std::fs::read_to_string(
        dir.path().join("test-app/shared/src/lib.rs"),
    )
    .unwrap();

    assert!(
        shared_lib.contains("hello_response_round_trips_through_json"),
        "shared lib should test HelloResponse round-trip"
    );
    assert!(
        shared_lib.contains("status_response_round_trips_through_json"),
        "shared lib should test StatusResponse round-trip"
    );
    assert!(
        shared_lib.contains("unknown_fields_are_ignored_not_rejected"),
        "shared lib should test unknown field tolerance"
    );
}

#[test]
fn scaffolded_shared_cargo_toml_has_serde_json_dev_dependency() {
    let dir = tempdir().unwrap();

    assert_cmd::Command::cargo_bin("flux-wasm-builder")
        .unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let cargo_toml = std::fs::read_to_string(
        dir.path().join("test-app/shared/Cargo.toml"),
    )
    .unwrap();

    assert!(
        cargo_toml.contains("[dev-dependencies]"),
        "shared Cargo.toml should have dev-dependencies section"
    );
    assert!(
        cargo_toml.contains("serde_json"),
        "shared Cargo.toml should have serde_json as dev-dependency"
    );
}

#[test]
fn scaffolded_frontend_fetches_api_status() {
    let dir = tempdir().unwrap();

    assert_cmd::Command::cargo_bin("flux-wasm-builder")
        .unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let frontend_lib = std::fs::read_to_string(
        dir.path().join("test-app/frontend/src/lib.rs"),
    )
    .unwrap();

    assert!(
        frontend_lib.contains("StatusResponse"),
        "frontend should import StatusResponse"
    );
    assert!(
        frontend_lib.contains("/api/status"),
        "frontend should fetch /api/status"
    );
}

#[test]
fn scaffolded_api_responses_use_json_content_type() {
    let dir = tempdir().unwrap();

    assert_cmd::Command::cargo_bin("flux-wasm-builder")
        .unwrap()
        .args(["init", "test-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    let api_mod = std::fs::read_to_string(
        dir.path().join("test-app/backend/src/api/mod.rs"),
    )
    .unwrap();

    // Both handlers use HttpResponse::Ok().json(...) which sets application/json
    assert!(
        api_mod.contains(".json(HelloResponse"),
        "hello handler should use .json() for content-type"
    );
    assert!(
        api_mod.contains(".json(StatusResponse"),
        "status handler should use .json() for content-type"
    );
}
```

---

### Step 3.6: Update Existing Scaffold Tests

**File:** `src/init/mod.rs`

Add a test in the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn shared_lib_has_compile_time_trait_assertions() {
    let root = tempdir().unwrap();
    scaffold(root.path(), "my-app").unwrap();
    let contents =
        std::fs::read_to_string(root.path().join("my-app/shared/src/lib.rs")).unwrap();
    assert!(contents.contains("assert_serialize"));
    assert!(contents.contains("assert_deserialize"));
    assert!(contents.contains("const _: fn()"));
}

#[test]
fn shared_cargo_toml_has_serde_json_dev_dependency() {
    let root = tempdir().unwrap();
    scaffold(root.path(), "my-app").unwrap();
    let contents =
        std::fs::read_to_string(root.path().join("my-app/shared/Cargo.toml")).unwrap();
    assert!(contents.contains("[dev-dependencies]"));
    assert!(contents.contains("serde_json"));
}

#[test]
fn backend_api_uses_status_response_not_json_macro() {
    let root = tempdir().unwrap();
    scaffold(root.path(), "my-app").unwrap();
    let contents =
        std::fs::read_to_string(root.path().join("my-app/backend/src/api/mod.rs")).unwrap();
    assert!(contents.contains("StatusResponse"));
    assert!(!contents.contains("json!"), "should use StatusResponse struct, not json!()");
}

#[test]
fn frontend_imports_and_fetches_status_response() {
    let root = tempdir().unwrap();
    scaffold(root.path(), "my-app").unwrap();
    let contents =
        std::fs::read_to_string(root.path().join("my-app/frontend/src/lib.rs")).unwrap();
    assert!(contents.contains("StatusResponse"));
    assert!(contents.contains("/api/status"));
}
```

---

## 4. Test Plan

### 4.1 Tests Within Generated Shared Crate (run by end-users)

| Test | Description |
|------|-------------|
| `hello_response_round_trips_through_json` | Serialize → deserialize HelloResponse |
| `status_response_round_trips_through_json` | Serialize → deserialize StatusResponse |
| `unknown_fields_are_ignored_not_rejected` | Deserialize with extra fields succeeds |

### 4.2 Compile-Time Checks Within Generated Shared Crate

| Check | Description |
|-------|-------------|
| `const _: fn() = \|\| { ... }` | Proves `Serialize` + `Deserialize` impls exist at compile time |

### 4.3 Tool Unit Tests (`src/init/mod.rs`)

| Test | Description |
|------|-------------|
| `shared_lib_has_compile_time_trait_assertions` | Generated shared/src/lib.rs contains assertions |
| `shared_cargo_toml_has_serde_json_dev_dependency` | Generated shared/Cargo.toml has serde_json dev-dep |
| `backend_api_uses_status_response_not_json_macro` | API handler uses typed struct |
| `frontend_imports_and_fetches_status_response` | Frontend fetches /api/status |

### 4.4 Tool Integration Tests (`tests/api.rs`)

| Test | Description |
|------|-------------|
| `scaffolded_api_mod_imports_status_response` | Imports both shared types |
| `scaffolded_api_status_uses_shared_type` | StatusResponse struct is constructed |
| `scaffolded_shared_lib_has_compile_time_assertions` | Compile-time checks present |
| `scaffolded_shared_lib_has_round_trip_tests` | All three unit tests present |
| `scaffolded_shared_cargo_toml_has_serde_json_dev_dependency` | dev-dep present |
| `scaffolded_frontend_fetches_api_status` | Frontend imports and fetches StatusResponse |
| `scaffolded_api_responses_use_json_content_type` | Both handlers use `.json()` |

---

## 5. Acceptance Criteria

| Criterion | Verification |
|-----------|--------------|
| All existing tests continue to pass | `cargo test` |
| New unit tests in `src/init/mod.rs` pass | `cargo test --lib` |
| New integration tests in `tests/api.rs` pass | `cargo test --test api` |
| Generated `shared/src/lib.rs` contains round-trip tests | Scaffold test |
| Generated `shared/src/lib.rs` contains compile-time assertions | Scaffold test |
| Generated `shared/src/lib.rs` does NOT contain `deny_unknown_fields` | Existing test |
| Generated `shared/Cargo.toml` has `serde_json` dev-dependency | Scaffold test |
| Generated `backend/src/api/mod.rs` uses `StatusResponse` from shared crate | Scaffold test |
| Generated `backend/src/api/mod.rs` does NOT use `serde_json::json!()` | Scaffold test |
| Generated `frontend/src/lib.rs` imports `StatusResponse` | Scaffold test |
| Generated `frontend/src/lib.rs` fetches `/api/status` | Scaffold test |
| API responses use `.json()` (which sets `application/json` content-type) | Scaffold test |

---

## 6. Implementation Order

1. **Update `shared_cargo_toml()` template** — Add `serde_json` dev-dependency
2. **Update `shared_lib_rs()` template** — Add tests and compile-time assertions
3. **Update `backend_api_mod()` template** — Use `StatusResponse` instead of `json!()`
4. **Update `frontend_lib_rs()` template** — Fetch `/api/status` and display version
5. **Add unit tests** to `src/init/mod.rs`
6. **Create `tests/api.rs`** — Integration tests for generated code
7. **Run full test suite** — `cargo test` and verify all pass

---

## 7. Files to Modify/Create

### Modified Files

| File | Changes |
|------|---------|
| `src/init/templates.rs` | Update 4 template functions: `shared_cargo_toml()`, `shared_lib_rs()`, `backend_api_mod()`, `frontend_lib_rs()` |
| `src/init/mod.rs` | Add 4 new unit tests to existing test module |

### New Files

| File | Purpose |
|------|---------|
| `tests/api.rs` | Integration tests for shared types integration |

### No Changes Required

| File | Reason |
|------|--------|
| `Cargo.toml` | No new tool dependencies needed |
| `src/lib.rs` | No new exports needed |
| `src/build/*` | Build subsystem is unchanged |
| `src/env_check.rs` | Environment checks unchanged |

---

## 8. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Template string escaping — the `const _: fn()` pattern with closures inside format strings | Use `r#"..."#` raw strings; the existing templates already handle this correctly |
| `serde_json` version conflict in generated workspace | Use `"1"` which is compatible with the backend's existing `serde_json = "1"` |
| Frontend compile failure on `StatusResponse` import | Verify `shared` crate compiles for `wasm32-unknown-unknown` (it only depends on `serde`, which supports wasm) |
| Existing test regression from template changes | Run `cargo test` before and after; template changes are additive (adding tests/assertions) or fixing correctness (StatusResponse) |

---

## 9. Scope Boundaries

### In Scope
- Template updates for shared types tests and assertions
- Using `StatusResponse` shared type in the status API handler
- Frontend fetching and displaying `/api/status`
- Scaffold tests verifying generated content

### Out of Scope
- Adding new shared types beyond `HelloResponse` and `StatusResponse`
- Backend hot reload (Section 9.3 open question)
- Build error overlay (Section 9.3 open question)
- WASM target cross-compilation tests (would require wasm-pack in CI)

---

*Plan created for Stage 6 — Shared Types Integration*
