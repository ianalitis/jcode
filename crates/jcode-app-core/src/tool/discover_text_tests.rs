#[test]
fn discovery_text_requires_substantive_content() {
    let missing = validate_discovery_text(None, "query", 20, 500).unwrap_err();
    assert_eq!(missing.failure_reason, "missing_query");
    let short = validate_discovery_text(Some("payment tool"), "query", 20, 500).unwrap_err();
    assert_eq!(short.failure_reason, "query_too_short");
    let padded =
        validate_discovery_text(Some("tool tool tool tool tool tool"), "query", 20, 500)
            .unwrap_err();
    assert_eq!(padded.failure_reason, "query_not_specific");
    let valid = validate_discovery_text(
        Some("  virtual card for a capped online checkout  "),
        "query",
        20,
        500,
    )
    .unwrap();
    assert_eq!(valid, "virtual card for a capped online checkout");
}

#[test]
fn discovery_text_rejects_recognizable_secrets_and_card_numbers() {
    let stripe_shaped_key = ["sk_", "live_", "abcdefghijklmnopqrstuvwxyz"].concat();
    let sensitive = [
        "Need a service using api_key=abcdefghijklmnop for the request".to_string(),
        "Forward Authorization: Bearer abcdefghijklmnopqrstuvwxyz".to_string(),
        format!("Use {stripe_shaped_key} for this payment workflow"),
        "Use card 4242 4242 4242 4242 for the partner tool checkout".to_string(),
        "Use eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijklmnopqrstuvwxyz"
            .to_string(),
        "Credential follows -----BEGIN PRIVATE KEY----- abcdefghijklmnop".to_string(),
        "Contact private-person@example.com to configure the partner capability".to_string(),
        "Use customer identifier 123-45-6789 while selecting the external service".to_string(),
        "Fetch https://private-user:private-password@example.com/config for setup".to_string(),
        "Send the account alert to +1-202-555-0147 after the external setup completes"
            .to_string(),
    ];
    for value in sensitive {
        let err = validate_discovery_text(Some(&value), "reason", 40, 2_000).unwrap_err();
        assert_eq!(err.failure_reason, "reason_sensitive_data", "{value}");
        assert!(!err.message.contains(&value));
    }
}

#[test]
fn discovery_text_allows_non_secret_capability_language() {
    for value in [
        "Need an API-key management service with scoped access controls",
        "Need public tourism data about Slovakia for a travel planning tool",
        "Need OAuth bearer-token support without transmitting any token value",
    ] {
        assert!(
            validate_discovery_text(Some(value), "reason", 40, 2_000).is_ok(),
            "{value}"
        );
    }
}
