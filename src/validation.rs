// src/validation.rs

//! Input validation for security-critical user inputs.
//!
//! This module provides validation functions to prevent injection attacks
//! and ensure user-provided values conform to expected formats.

/// Error type for validation failures.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// The project name is empty.
    #[error("project name is empty")]
    Empty,

    /// The project name contains invalid characters.
    #[error("project name '{0}' contains invalid characters")]
    InvalidCharacters(String),

    /// The project name exceeds the maximum length.
    #[error("project name '{0}' exceeds maximum length of {1} characters")]
    TooLong(String, usize),

    /// The project name is a reserved word.
    #[error("project name '{0}' is a reserved word")]
    ReservedWord(String),

    /// The project name starts with an invalid character.
    #[error("project name '{0}' cannot start with '{1}'")]
    InvalidPrefix(String, char),
}

/// Validate a project name according to Cargo conventions.
///
/// # Rules
///
/// - Must be 1-64 characters
/// - Only alphanumeric, hyphens, and underscores allowed
/// - Cannot start with hyphen or underscore
/// - Cannot be a reserved Rust keyword
/// - Cannot be a reserved Cargo command name
/// - Cannot be a reserved Windows filename
///
/// # Security
///
/// This validation prevents injection attacks by ensuring the project name
/// only contains safe characters that won't break out of generated code
/// contexts (TOML, Rust, HTML).
///
/// # Examples
///
/// ```
/// use flux_wasm_builder::validation::validate_project_name;
///
/// assert!(validate_project_name("my-app").is_ok());
/// assert!(validate_project_name("my_app").is_ok());
/// assert!(validate_project_name("").is_err());
/// assert!(validate_project_name("test\"injection").is_err());
/// ```
pub fn validate_project_name(name: &str) -> Result<(), ValidationError> {
    // Check empty
    if name.is_empty() {
        return Err(ValidationError::Empty);
    }

    // Check length (Cargo max is 64)
    const MAX_LENGTH: usize = 64;
    if name.len() > MAX_LENGTH {
        return Err(ValidationError::TooLong(name.to_string(), MAX_LENGTH));
    }

    // Check first character (cannot start with hyphen or underscore)
    let first_char = name.chars().next().unwrap();
    if first_char == '-' || first_char == '_' {
        return Err(ValidationError::InvalidPrefix(name.to_string(), first_char));
    }

    // Check valid characters (alphanumeric, hyphen, underscore only)
    for c in name.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' {
            return Err(ValidationError::InvalidCharacters(name.to_string()));
        }
    }

    // Check reserved words (Rust keywords and Cargo commands)
    const RESERVED_WORDS: &[&str] = &[
        // Cargo commands
        "test", "build", "run", "check", "doc", "publish",
        "new", "init", "install", "uninstall", "search", "clean",
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

    // Check Windows reserved filenames
    const WINDOWS_RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let upper_name = name.to_uppercase();
    for reserved in WINDOWS_RESERVED {
        if upper_name == *reserved {
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
        assert!(validate_project_name("A").is_ok());
        assert!(validate_project_name("My-App_123").is_ok());
    }

    #[test]
    fn rejects_empty_name() {
        assert!(matches!(validate_project_name(""), Err(ValidationError::Empty)));
    }

    #[test]
    fn rejects_too_long_name() {
        let long_name = "a".repeat(65);
        assert!(matches!(
            validate_project_name(&long_name),
            Err(ValidationError::TooLong(_, 64))
        ));
    }

    #[test]
    fn accepts_max_length_name() {
        let max_name = "a".repeat(64);
        assert!(validate_project_name(&max_name).is_ok());
    }

    #[test]
    fn rejects_invalid_first_char() {
        assert!(matches!(
            validate_project_name("-test"),
            Err(ValidationError::InvalidPrefix(_, '-'))
        ));
        assert!(matches!(
            validate_project_name("_test"),
            Err(ValidationError::InvalidPrefix(_, '_'))
        ));
    }

    #[test]
    fn rejects_invalid_characters() {
        // Space
        assert!(matches!(
            validate_project_name("test app"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        // Quote
        assert!(matches!(
            validate_project_name("test\"app"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        // Newline
        assert!(matches!(
            validate_project_name("test\napp"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        // Tab
        assert!(matches!(
            validate_project_name("test\tapp"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        // Special chars
        assert!(matches!(
            validate_project_name("test/app"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        assert!(matches!(
            validate_project_name("test\\app"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        assert!(matches!(
            validate_project_name("test.app"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        assert!(matches!(
            validate_project_name("test@app"),
            Err(ValidationError::InvalidCharacters(_))
        ));
    }

    #[test]
    fn rejects_reserved_rust_keywords() {
        assert!(matches!(
            validate_project_name("fn"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("struct"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("if"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("true"),
            Err(ValidationError::ReservedWord(_))
        ));
    }

    #[test]
    fn rejects_reserved_cargo_commands() {
        assert!(matches!(
            validate_project_name("test"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("build"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("run"),
            Err(ValidationError::ReservedWord(_))
        ));
    }

    #[test]
    fn rejects_windows_reserved_names() {
        assert!(matches!(
            validate_project_name("CON"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("con"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("NUL"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("COM1"),
            Err(ValidationError::ReservedWord(_))
        ));
        assert!(matches!(
            validate_project_name("LPT1"),
            Err(ValidationError::ReservedWord(_))
        ));
    }

    #[test]
    fn rejects_injection_attempts() {
        // TOML injection
        assert!(matches!(
            validate_project_name("test\"\n[dependencies]\nevil=\"1\""),
            Err(ValidationError::InvalidCharacters(_))
        ));
        // HTML injection
        assert!(matches!(
            validate_project_name("test</title><script>alert(1)</script>"),
            Err(ValidationError::InvalidCharacters(_))
        ));
        // Path traversal
        assert!(matches!(
            validate_project_name("../etc/passwd"),
            Err(ValidationError::InvalidCharacters(_))
        ));
    }
}
