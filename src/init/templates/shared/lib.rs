use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }
    }
}

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
    assert_serialize::<ApiResponse<()>>();
    assert_deserialize::<ApiResponse<()>>();
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
        // Verifies forward-compatibility: extra fields are silently ignored
        let json = r#"{"message":"hi","future_field":true}"#;
        let r: HelloResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.message, "hi");
    }
}