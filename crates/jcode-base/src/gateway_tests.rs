use super::*;

#[test]
fn gateway_defaults_to_loopback() {
    assert_eq!(GatewayConfig::default().bind_addr, "127.0.0.1");
    assert_eq!(
        crate::config::Config::default().gateway.bind_addr,
        "127.0.0.1"
    );
}
use tokio_tungstenite::tungstenite::handshake::server::Request;

/// `DeviceRegistry` persists every pairing change to `$JCODE_HOME/devices.json`
/// (default `~/.jcode`). Without this, registry tests paired a fake
/// "Test iPhone" into the developer's real device registry on every run.
struct TempHome {
    _lock: crate::storage::TestEnvGuard,
    previous: Option<std::ffi::OsString>,
    _temp: tempfile::TempDir,
}

impl TempHome {
    fn new() -> Self {
        let lock = crate::storage::lock_test_env();
        let temp = tempfile::tempdir().expect("temp dir");
        let previous = std::env::var_os("JCODE_HOME");
        crate::env::set_var("JCODE_HOME", temp.path());
        Self {
            _lock: lock,
            previous,
            _temp: temp,
        }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => crate::env::set_var("JCODE_HOME", value),
            None => crate::env::remove_var("JCODE_HOME"),
        }
    }
}

#[test]
fn test_device_registry_pairing() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();

    // Generate pairing code
    let code = registry.generate_pairing_code();
    assert_eq!(code.len(), 6);
    assert_eq!(registry.pending_codes.len(), 1);

    // Validate correct code
    assert!(registry.validate_code(&code));
    assert_eq!(registry.pending_codes.len(), 0); // consumed

    // Validate again should fail (consumed)
    assert!(!registry.validate_code(&code));
}

/// A different 6-digit code than `code`, so the guess is always wrong.
fn wrong_code(code: &str) -> String {
    let n: u32 = code.parse().unwrap();
    format!("{:06}", (n + 1) % 1_000_000)
}

#[test]
fn failed_pairing_guesses_revoke_pending_codes_at_the_cap() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();
    let code = registry.generate_pairing_code();

    // Each request reloads the registry, so every guess must persist its count.
    for attempt in 1..=registry::MAX_FAILED_PAIRING_ATTEMPTS {
        let mut per_request = DeviceRegistry::load();
        assert!(!per_request.validate_code(&wrong_code(&code)));
        let stored = DeviceRegistry::load();
        assert_eq!(stored.failed_pairing_attempts, attempt);
        if attempt < registry::MAX_FAILED_PAIRING_ATTEMPTS {
            assert_eq!(stored.pending_codes.len(), 1);
            assert_eq!(stored.pending_codes[0].code, code);
        }
    }
    let mut registry = DeviceRegistry::load();
    assert!(registry.pending_codes.is_empty());
    // The real code no longer works: enumeration cannot outlast the cap.
    assert!(!registry.validate_code(&code));

    // The counter survives the reload the gateway does on every request.
    let reloaded = DeviceRegistry::load();
    assert!(reloaded.pending_codes.is_empty());

    // A freshly issued code resets the budget and works.
    let fresh = registry.generate_pairing_code();
    assert_eq!(registry.failed_pairing_attempts, 0);
    assert!(registry.validate_code(&fresh));
}

#[test]
fn a_few_typos_still_allow_pairing() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();
    let code = registry.generate_pairing_code();

    for _ in 1..registry::MAX_FAILED_PAIRING_ATTEMPTS {
        assert!(!registry.validate_code(&wrong_code(&code)));
    }
    let mut reloaded = DeviceRegistry::load();
    assert!(reloaded.validate_code(&code));
    assert_eq!(reloaded.failed_pairing_attempts, 0);
}

/// A readable registry whose write fails before truncation, not a transaction.
/// Declare this guard after `TempHome` so permissions are restored first.
#[cfg(unix)]
struct ReadOnlyRegistry {
    path: std::path::PathBuf,
    permissions: std::fs::Permissions,
    original: Vec<u8>,
    code: String,
}

#[cfg(unix)]
impl ReadOnlyRegistry {
    fn new(home: &TempHome) -> Self {
        use std::os::unix::fs::PermissionsExt;

        let mut registry = DeviceRegistry::default();
        let code = registry.generate_pairing_code();
        registry.failed_pairing_attempts = 2;
        registry.save().expect("seed registry saves successfully");
        let path = home._temp.path().join("devices.json");
        let fixture = Self {
            permissions: std::fs::metadata(&path).unwrap().permissions(),
            original: std::fs::read(&path).unwrap(),
            path,
            code,
        };
        fixture.assert_disk_unchanged();
        std::fs::set_permissions(&fixture.path, std::fs::Permissions::from_mode(0o444)).unwrap();
        assert!(
            std::fs::OpenOptions::new()
                .write(true)
                .open(&fixture.path)
                .is_err(),
            "fixture requires a real write-open failure, including on privileged runners"
        );
        assert!(DeviceRegistry::load().save().is_err());
        fixture.assert_disk_unchanged();
        fixture
    }

    fn assert_disk_unchanged(&self) {
        assert_eq!(std::fs::read(&self.path).unwrap(), self.original);
        let stored = DeviceRegistry::load();
        assert_eq!(stored.pending_codes.len(), 1);
        assert_eq!(stored.pending_codes[0].code, self.code);
        assert!(stored.pending_codes[0].expires_at > chrono::Utc::now().to_rfc3339());
        assert_eq!(stored.failed_pairing_attempts, 2);
        assert!(stored.devices.is_empty());
    }
}

#[cfg(unix)]
impl Drop for ReadOnlyRegistry {
    fn drop(&mut self) {
        std::fs::set_permissions(&self.path, self.permissions.clone())
            .expect("restore disposable registry permissions");
    }
}

#[cfg(unix)]
#[test]
fn pairing_fails_closed_when_registry_cannot_be_saved() {
    let home = TempHome::new();
    let fixture = ReadOnlyRegistry::new(&home);
    let mut registry = DeviceRegistry::load();

    let accepted = registry.validate_code(&fixture.code);
    assert!(registry.pending_codes.is_empty());
    assert_eq!(registry.failed_pairing_attempts, 0);
    fixture.assert_disk_unchanged();
    assert!(
        !accepted,
        "a correct code must not pair when its consumption cannot be saved"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn pair_request_refuses_unsaved_code_consumption() {
    let home = TempHome::new();
    let fixture = ReadOnlyRegistry::new(&home);
    let registry = Arc::new(tokio::sync::RwLock::new(DeviceRegistry::load()));
    let body = serde_json::json!({
        "code": fixture.code,
        "device_id": "test-device",
        "device_name": "Test device",
    })
    .to_string();

    // The handler reloads the still-readable disk code on both requests.
    for _ in 0..2 {
        let response = handle_pair_request(&body, &registry).await;
        let response = std::str::from_utf8(&response).unwrap();
        assert!(response.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
        let (_, body) = response.split_once("\r\n\r\n").unwrap();
        let body: serde_json::Value = serde_json::from_str(body).unwrap();
        assert!(body.get("token").is_none());
        let in_memory = registry.read().await;
        assert!(in_memory.devices.is_empty());
        assert!(in_memory.pending_codes.is_empty());
        assert_eq!(in_memory.failed_pairing_attempts, 0);
        fixture.assert_disk_unchanged();
    }
}

#[tokio::test]
async fn pair_request_success_persists_token_and_consumes_code() {
    let home = TempHome::new();
    let mut seed = DeviceRegistry::default();
    let code = seed.generate_pairing_code();
    seed.save().expect("seed registry saves successfully");
    let registry = Arc::new(tokio::sync::RwLock::new(DeviceRegistry::load()));
    let body = serde_json::json!({
        "code": code,
        "device_id": "test-device",
        "device_name": "Test device",
    })
    .to_string();

    let response = handle_pair_request(&body, &registry).await;
    let response = std::str::from_utf8(&response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    let (_, response_body) = response.split_once("\r\n\r\n").unwrap();
    let response_body: serde_json::Value = serde_json::from_str(response_body).unwrap();
    let token = response_body["token"].as_str().expect("pairing token");
    let stored = DeviceRegistry::load();
    assert_eq!(stored.validate_token(token).unwrap().id, "test-device");
    assert_eq!(stored.devices.len(), 1);
    assert!(stored.pending_codes.is_empty());
    assert_eq!(stored.failed_pairing_attempts, 0);
    let disk = std::fs::read_to_string(home._temp.path().join("devices.json")).unwrap();
    assert!(!disk.contains(token));

    let replay = handle_pair_request(&body, &registry).await;
    let replay = std::str::from_utf8(&replay).unwrap();
    assert!(replay.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
    let (_, replay_body) = replay.split_once("\r\n\r\n").unwrap();
    let replay_body: serde_json::Value = serde_json::from_str(replay_body).unwrap();
    assert!(replay_body.get("token").is_none());
    assert!(DeviceRegistry::load().validate_token(token).is_some());
}

#[cfg(unix)]
#[tokio::test]
async fn pair_registration_refuses_unsaved_device() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempHome::new();
    let mut seed = DeviceRegistry::default();
    let code = seed.generate_pairing_code();
    seed.failed_pairing_attempts = 2;
    assert!(seed.save().is_ok());
    let mut reg = DeviceRegistry::load();
    assert_eq!(reg.pending_codes.len(), 1);
    assert!(reg.pending_codes[0].code == code);
    assert_eq!(reg.failed_pairing_attempts, 2);
    assert!(reg.validate_code(&code));
    let stored = DeviceRegistry::load();
    assert!(stored.pending_codes.is_empty());
    assert_eq!(stored.failed_pairing_attempts, 0);
    assert!(stored.devices.is_empty());

    // Reuse only the permission restoration guard, after persisted consumption.
    let path = home._temp.path().join("devices.json");
    let fixture = ReadOnlyRegistry {
        permissions: std::fs::metadata(&path).unwrap().permissions(),
        original: std::fs::read(&path).unwrap(),
        path,
        code: code.clone(),
    };
    std::fs::set_permissions(&fixture.path, std::fs::Permissions::from_mode(0o444)).unwrap();
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&fixture.path)
            .is_err(),
        "fixture requires a real write-open failure, including on privileged runners"
    );
    assert!(reg.save().is_err());

    // Exercise the exact production block, not an injected filesystem race.
    let response = complete_pairing_registration(
        &mut reg,
        "test-device".to_string(),
        "Test device".to_string(),
        None,
    );
    assert!(std::fs::read(&fixture.path).unwrap() == fixture.original);
    let stored = DeviceRegistry::load();
    assert!(stored.devices.is_empty());
    assert!(stored.pending_codes.is_empty());
    assert_eq!(stored.failed_pairing_attempts, 0);
    let response = std::str::from_utf8(&response).unwrap_or_else(|_| panic!("HTTP text"));
    let (headers, response_body) = response.split_once("\r\n\r\n").expect("HTTP body");
    let status: u16 = headers
        .split_whitespace()
        .nth(1)
        .expect("HTTP status")
        .parse()
        .unwrap_or_else(|_| panic!("numeric HTTP status"));
    let response_body: serde_json::Value =
        serde_json::from_str(response_body).unwrap_or_else(|_| panic!("JSON response"));
    if let Some(token) = response_body.get("token").and_then(|value| value.as_str()) {
        match auth::authorize_ws_device(&stored, token) {
            Ok(_) => panic!("unsaved registration must not authorize from fresh disk"),
            Err(error) => assert_eq!(error.status(), 401),
        }
    }

    let registry = Arc::new(tokio::sync::RwLock::new(DeviceRegistry::load()));
    let body = serde_json::json!({
        "code": code,
        "device_id": "test-device",
        "device_name": "Test device",
    })
    .to_string();
    let replay = handle_pair_request(&body, &registry).await;
    let replay = std::str::from_utf8(&replay).unwrap_or_else(|_| panic!("HTTP replay text"));
    assert!(replay.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
    let (_, replay_body) = replay.split_once("\r\n\r\n").expect("HTTP replay body");
    let replay_body: serde_json::Value =
        serde_json::from_str(replay_body).unwrap_or_else(|_| panic!("JSON replay response"));
    assert!(replay_body.get("token").is_none());
    assert!(std::fs::read(&fixture.path).unwrap() == fixture.original);

    assert_eq!(status, 500, "registration must report persistence failure");
    assert!(headers.starts_with("HTTP/1.1 500 Internal Server Error\r\n"));
    assert!(response_body.get("token").is_none());
    assert!(
        response_body == serde_json::json!({"error": "Pairing registration could not be saved"})
    );
}

#[test]
fn guesses_without_pending_codes_are_not_counted() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();
    assert!(!registry.validate_code("123456"));
    assert_eq!(registry.failed_pairing_attempts, 0);
}

#[test]
fn test_device_registry_token_auth() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();

    // Pair a device
    let token = registry
        .pair_device("test-device-1".to_string(), "Test iPhone".to_string(), None)
        .unwrap();

    // Validate correct token
    assert!(registry.validate_token(&token).is_some());
    let device = registry.validate_token(&token).unwrap();
    assert_eq!(device.name, "Test iPhone");
    assert_eq!(device.id, "test-device-1");

    // Validate wrong token
    assert!(registry.validate_token("wrong-token").is_none());

    // Token hash should be stored, not raw token
    assert!(registry.devices[0].token_hash.starts_with("sha256:"));
}

#[test]
fn test_device_re_pairing() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();

    // Pair same device twice
    let token1 = registry
        .pair_device("device-1".to_string(), "iPhone v1".to_string(), None)
        .unwrap();
    let token2 = registry
        .pair_device("device-1".to_string(), "iPhone v2".to_string(), None)
        .unwrap();

    // Only one device entry (old one replaced)
    assert_eq!(registry.devices.len(), 1);
    assert_eq!(registry.devices[0].name, "iPhone v2");

    // Old token should be invalid
    assert!(registry.validate_token(&token1).is_none());
    // New token should be valid
    assert!(registry.validate_token(&token2).is_some());
}

#[test]
fn test_parse_bearer_token() {
    assert_eq!(parse_bearer_token("Bearer abc"), Some("abc"));
    assert_eq!(parse_bearer_token("bearer abc"), Some("abc"));
    assert_eq!(parse_bearer_token("BEARER abc"), Some("abc"));
    assert_eq!(parse_bearer_token("Bearer"), None);
    assert_eq!(parse_bearer_token("Basic abc"), None);
    assert_eq!(parse_bearer_token("Bearer abc def"), None);
}

#[test]
fn test_parse_query_token() {
    assert_eq!(parse_query_token("token=abc"), Some("abc"));
    assert_eq!(parse_query_token("foo=bar&token=abc123"), Some("abc123"));
    assert_eq!(parse_query_token("token="), None);
    assert_eq!(parse_query_token("foo=bar"), None);
}

#[test]
fn test_hex_token_validation() {
    assert!(is_valid_hex_token(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    ));
    assert!(!is_valid_hex_token("abc"));
    assert!(!is_valid_hex_token(
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
    ));
}

#[test]
fn test_extract_ws_auth_prefers_header_and_falls_back_to_query() {
    let token_a = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let token_b = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    let header_request = Request::builder()
        .uri("ws://example.com/ws")
        .header("authorization", format!("Bearer {token_a}"))
        .body(())
        .expect("request");
    let header_auth = extract_ws_auth(&header_request).expect("header auth");
    assert_eq!(header_auth.token, token_a);
    assert_eq!(header_auth.source, WsAuthSource::Header);

    let query_request = Request::builder()
        .uri(format!("ws://example.com/ws?token={token_b}"))
        .body(())
        .expect("request");
    let query_auth = extract_ws_auth(&query_request).expect("query auth");
    assert_eq!(query_auth.token, token_b);
    assert_eq!(query_auth.source, WsAuthSource::Query);
}

#[test]
fn test_extract_ws_auth_rejects_conflicting_sources() {
    let token_a = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let token_b = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    let request = Request::builder()
        .uri(format!("ws://example.com/ws?token={token_b}"))
        .header("authorization", format!("Bearer {token_a}"))
        .body(())
        .expect("request");
    assert!(extract_ws_auth(&request).is_err());
}

#[test]
fn test_find_header_end() {
    assert_eq!(
        super::find_header_end(b"POST /pair HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}"),
        Some(38)
    );
    assert_eq!(
        super::find_header_end(b"POST /pair HTTP/1.1\r\nContent-"),
        None
    );
    assert_eq!(super::find_header_end(b""), None);
}

#[test]
fn test_authorize_ws_device_valid_token() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();
    let token = registry
        .pair_device("dev-1".to_string(), "iPhone".to_string(), None)
        .unwrap();

    let device = auth::authorize_ws_device(&registry, &token).expect("valid token authorizes");
    assert_eq!(device.name, "iPhone");
    assert_eq!(device.id, "dev-1");
}

#[test]
fn test_authorize_ws_device_rejects_unknown_and_revoked_with_401() {
    let _home = TempHome::new();
    let mut registry = DeviceRegistry::default();
    let token = registry
        .pair_device("dev-1".to_string(), "iPhone".to_string(), None)
        .unwrap();

    // Unknown token -> 401 at handshake time.
    let unknown = "a".repeat(64);
    let err =
        auth::authorize_ws_device(&registry, &unknown).expect_err("unknown token must be rejected");
    assert_eq!(err.status(), 401);
    assert!(
        err.body()
            .as_deref()
            .unwrap_or_default()
            .contains("re-pair"),
        "401 body should tell the client to re-pair"
    );

    // Revoked device -> same 401 path.
    registry.devices.retain(|d| d.id != "dev-1");
    let err =
        auth::authorize_ws_device(&registry, &token).expect_err("revoked token must be rejected");
    assert_eq!(err.status(), 401);
}
