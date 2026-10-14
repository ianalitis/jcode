use super::*;

fn card(prefix: &str, length: usize) -> String {
    let mut number = format!("{prefix}{}", "0".repeat(length - prefix.len() - 1));
    let sum: u32 = number
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let n = (c - b'0') * if i % 2 == 0 { 2 } else { 1 };
            u32::from(if n > 9 { n - 9 } else { n })
        })
        .sum();
    number.push(char::from(b'0' + ((10 - sum % 10) % 10) as u8));
    number
}

#[test]
fn multiple_findings_one_line_and_idempotence() {
    let key = ["sk", &"a".repeat(24)].join("-");
    let first = card("4", 16);
    let second = card("37", 15);
    let input =
        format!("first={first}, second={second}; key={key} TOKEN='opaque value' PASSWORD=other");
    let clean = sanitize_outbound_text(&input).unwrap();
    for value in [&first, &second, &key] {
        assert!(!clean.contains(value));
    }
    assert_eq!(clean.matches(CARD).count(), 2);
    assert!(!clean.contains("opaque value"));
    assert!(!clean.contains("other"));
    assert!(sanitize_outbound_text(&clean).unwrap() == clean);
}

#[test]
fn cards_require_issuer_grouping_and_luhn() {
    for (prefix, length) in [
        ("4", 13),
        ("4", 16),
        ("4", 19),
        ("51", 16),
        ("2221", 16),
        ("37", 15),
        ("36", 14),
        ("6011", 16),
        ("65", 19),
        ("3528", 16),
        ("62", 19),
    ] {
        let number = card(prefix, length);
        assert!(sanitize_outbound_text(&number).unwrap() == CARD);
        if length == 16 || length == 19 {
            for sep in [" ", "-"] {
                let grouped = number
                    .as_bytes()
                    .chunks(4)
                    .map(|part| std::str::from_utf8(part).unwrap())
                    .collect::<Vec<_>>()
                    .join(sep);
                assert!(sanitize_outbound_text(&grouped).unwrap() == CARD);
            }
        }
    }
    let amex = card("34", 15);
    let grouped = format!("{} {} {}", &amex[..4], &amex[4..10], &amex[10..]);
    assert!(sanitize_outbound_text(&grouped).unwrap() == CARD);
    let mut invalid = card("4", 16);
    let last = invalid.pop().unwrap();
    invalid.push(if last == '9' {
        '0'
    } else {
        char::from(last as u8 + 1)
    });
    for benign in [
        invalid,
        card("9", 16),
        ["1001", "1002", "1003", "1004"].join(" "),
        format!("id_{}", card("4", 16)),
        "1234 5678 9012".into(),
        "2026-10-12".into(),
    ] {
        assert!(sanitize_outbound_text(&benign).unwrap() == benign);
    }
}

#[test]
fn nested_json_protects_restored_arguments_reasoning_extensions_and_keys() {
    let number = card("4", 16);
    let escaped: String = number
        .chars()
        .map(|c| format!("\\u{:04x}", c as u32))
        .collect();
    let arguments = format!("{{\"account\":\"{escaped}\",\"password\":\"opaque\"}}");
    let mut value = serde_json::json!({
        "messages": [{"tool_calls": [{"function": {"arguments": arguments}}], "reasoning_content": number}],
        "extension": [{"nested": number}], "max_tokens": 4096, "stream": true,
        "api_key": "opaque", "benign": [1001, 1002, 1003, 1004]
    });
    value
        .as_object_mut()
        .unwrap()
        .insert(number.clone(), Value::String(number.clone()));
    sanitize_outbound_json(&mut value).unwrap();
    assert!(!value.to_string().contains(&number));
    assert!(!value.to_string().contains("opaque"));
    assert_eq!(value["max_tokens"], 4096);
    assert_eq!(value["stream"], true);
    assert_eq!(value["benign"], serde_json::json!([1001, 1002, 1003, 1004]));
    assert_eq!(value[CARD], CARD);
    let decoded: Value = serde_json::from_str(
        value["messages"][0]["tool_calls"][0]["function"]["arguments"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(decoded["account"], CARD);
    assert_eq!(decoded["password"], SECRET);
}

#[test]
fn custom_assignment_without_old_fast_path_keywords() {
    let sanitizer = Sanitizer {
        rules: Sanitizer::new().unwrap().rules,
        custom: vec![(
            "CUSTOM_CREDENTIAL".into(),
            assignment_pattern("CUSTOM_CREDENTIAL").unwrap(),
        )],
    };
    let clean = sanitizer
        .text("prefix CUSTOM_CREDENTIAL=opaque suffix", 0)
        .unwrap();
    assert_eq!(clean, "prefix CUSTOM_CREDENTIAL=[REDACTED_SECRET] suffix");
}

#[test]
fn failures_are_metadata_only_and_never_silently_skip_rules() {
    let error = compile("(").unwrap_err();
    assert_eq!(
        error.to_string(),
        "outbound sanitizer failed; content withheld"
    );
    let mut value = Value::String("opaque".into());
    for _ in 0..=MAX_DEPTH {
        value = Value::Array(vec![value]);
    }
    assert!(sanitize_outbound_json(&mut value).is_err());
    let mut colliding = serde_json::json!({});
    colliding[card("4", 16)] = Value::Null;
    colliding[card("51", 16)] = Value::Null;
    assert!(sanitize_outbound_json(&mut colliding).is_err());
}

#[test]
fn truncated_private_keys_and_additional_token_families_are_protected() {
    for kind in ["", "RSA ", "EC ", "OPENSSH ", "DSA ", "PGP ", "ENCRYPTED "] {
        let header = ["-----BEGIN ", kind, "PRIVATE", " KEY-----"].concat();
        let material = "sensitive-material".repeat(4);
        let text = format!("before\n{header}\n{material}");
        let clean = sanitize_outbound_text(&text).unwrap();
        assert!(!clean.contains(&header) && !clean.contains(&material));
        assert!(clean == format!("before\n{SECRET}"));
    }
    for family in ["p", "o", "u", "s", "r"] {
        let token = ["gh", family, "_", &"a".repeat(36)].concat();
        assert!(sanitize_outbound_text(&token).unwrap() == SECRET);
    }
    let temporary_key = ["AS", "IA", &"Z".repeat(16)].concat();
    assert!(sanitize_outbound_text(&temporary_key).unwrap() == SECRET);
}

#[test]
fn complete_header_values_are_redacted_not_just_the_first_word() {
    let first = ["opaque", "-credential"].concat();
    let second = ["other", "-credential"].concat();
    for text in [
        format!("Authorization: Basic {first}"),
        format!("Cookie: session={first}; other={second}"),
        format!("Set-Cookie: session={first}; Path=/"),
    ] {
        let clean = sanitize_outbound_text(&text).unwrap();
        assert!(!clean.contains(&first) && !clean.contains(&second));
        assert!(sanitize_outbound_text(&clean).unwrap() == clean);
    }
}

#[test]
fn numeric_json_pan_and_longest_grouping_are_protected_without_masking_ids() {
    let long = card("4", 19);
    let pan: u64 = long.parse().unwrap();
    let benign: u64 = card("9", 16).parse().unwrap();
    let mut value =
        serde_json::json!({"number": pan, "id": benign, "ids": [1001, 1002, 1003, 1004]});
    sanitize_outbound_json(&mut value).unwrap();
    assert!(value["number"] == CARD);
    assert!(value["id"] == benign);
    assert_eq!(value["ids"], serde_json::json!([1001, 1002, 1003, 1004]));
    for length in [17, 18, 19] {
        let number = card("3528", length);
        let grouped = number
            .as_bytes()
            .chunks(4)
            .map(|part| std::str::from_utf8(part).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(sanitize_outbound_text(&grouped).unwrap() == CARD);
    }
    let short = card("4", 16);
    let grouped = short
        .as_bytes()
        .chunks(4)
        .map(|part| std::str::from_utf8(part).unwrap())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        sanitize_outbound_text(&format!("{grouped} {grouped}")).unwrap()
            == format!("{CARD} {CARD}")
    );
    let too_long = format!("{long}0");
    assert!(sanitize_outbound_text(&too_long).unwrap() == too_long);
}
