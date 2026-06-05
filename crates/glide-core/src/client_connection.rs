use serde_json::{json, Value};

/// Build the device registration payload shared by CLI and GUI clients.
pub fn registration_body(
    device_id: &str,
    device_name: &str,
    platform: &str,
    registration_token: Option<&str>,
) -> Value {
    let mut body = json!({
        "device_id": device_id,
        "name": device_name,
        "platform": platform,
        "trusted": true,
    });

    if let Some(token) = registration_token.filter(|token| !token.trim().is_empty()) {
        body["registration_token"] = Value::String(token.to_string());
    }

    body
}

/// Return a non-sensitive token marker for diagnostics.
pub fn mask_secret(secret: Option<&str>) -> String {
    match secret.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.len() > 8 => format!("{}...", &value[..4]),
        Some(_) => "***".to_string(),
        None => "not set".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{mask_secret, registration_body};

    #[test]
    fn registration_body_includes_registration_token_when_provided() {
        let body = registration_body("dev-1", "WinClient", "windows", Some("reg123"));

        assert_eq!(body["device_id"], "dev-1");
        assert_eq!(body["name"], "WinClient");
        assert_eq!(body["platform"], "windows");
        assert_eq!(body["registration_token"], "reg123");
    }

    #[test]
    fn registration_body_omits_empty_registration_token() {
        let body = registration_body("dev-1", "WinClient", "windows", Some("  "));

        assert!(body.get("registration_token").is_none());
    }

    #[test]
    fn mask_secret_does_not_expose_full_token() {
        assert_eq!(mask_secret(Some("secret-token-value")), "secr...");
        assert_eq!(mask_secret(Some("short")), "***");
        assert_eq!(mask_secret(None), "not set");
    }
}
