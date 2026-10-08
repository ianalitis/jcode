use super::*;

#[test]
fn ascii_subject_is_unchanged() {
    assert_eq!(encode_header_value("plain subject"), "plain subject");
}

#[test]
fn non_ascii_subject_is_rfc2047_encoded() {
    let encoded = encode_header_value("Re: pricing \u{2014} inquiry");
    assert!(encoded.starts_with("=?UTF-8?B?"), "got: {encoded}");
    assert!(encoded.ends_with("?="));
    // Round-trip: decoding the base64 payload restores the original.
    let payload = encoded
        .trim_start_matches("=?UTF-8?B?")
        .trim_end_matches("?=");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "Re: pricing \u{2014} inquiry"
    );
}

#[test]
fn long_non_ascii_subject_folds_into_multiple_encoded_words() {
    let long = "\u{2014}".repeat(60);
    let encoded = encode_header_value(&long);
    for word in encoded.split("\r\n ") {
        assert!(word.len() <= 75, "encoded word too long: {}", word.len());
        assert!(word.starts_with("=?UTF-8?B?") && word.ends_with("?="));
    }
    assert!(encoded.split("\r\n ").count() > 1);
}

#[test]
fn reply_headers_use_angle_bracketed_message_id() {
    let raw = build_raw_mime(
        &Recipients::to("a@b.c"),
        "Re: hi",
        "body",
        Some("CAOU+8LMfxVaPMmigYAtdJK0z0Y@mail.gmail.com"),
        &[],
    )
    .unwrap();
    assert!(raw.contains("In-Reply-To: <CAOU+8LMfxVaPMmigYAtdJK0z0Y@mail.gmail.com>"));
    assert!(raw.contains("References: <CAOU+8LMfxVaPMmigYAtdJK0z0Y@mail.gmail.com>"));
    // Already-bracketed IDs are not double wrapped.
    let raw2 = build_raw_mime(
        &Recipients::to("a@b.c"),
        "Re: hi",
        "body",
        Some("<x@y.z>"),
        &[],
    )
    .unwrap();
    assert!(raw2.contains("In-Reply-To: <x@y.z>"));
    assert!(!raw2.contains("<<"));
}

#[test]
fn utf8_subject_in_raw_mime_is_ascii_only() {
    let raw = build_raw_mime(
        &Recipients::to("a@b.c"),
        "caf\u{e9} \u{2014} r\u{e9}sum\u{e9}",
        "body",
        None,
        &[],
    )
    .unwrap();
    let subject_line = raw.lines().find(|l| l.starts_with("Subject:")).unwrap();
    assert!(subject_line.is_ascii(), "subject header leaked raw UTF-8");
}

#[test]
fn cc_and_bcc_headers_are_written_when_present() {
    let recipients = Recipients {
        to: "david@hey.com",
        cc: Some("cc@x.y"),
        bcc: Some("bflora@ycombinator.com, other@x.y"),
    };
    let raw = build_raw_mime(&recipients, "Re: intro", "body", None, &[]).unwrap();
    let headers = raw.split("\r\n\r\n").next().unwrap();
    assert!(headers.contains("To: david@hey.com\r\n"));
    assert!(headers.contains("Cc: cc@x.y\r\n"));
    assert!(headers.contains("Bcc: bflora@ycombinator.com, other@x.y\r\n"));

    // Multipart messages carry the same address headers.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "hi").unwrap();
    let raw = build_raw_mime(&recipients, "s", "b", None, &[file]).unwrap();
    let headers = raw.split("\r\n\r\n").next().unwrap();
    assert!(headers.contains("Bcc: bflora@ycombinator.com, other@x.y\r\n"));
}

#[test]
fn empty_cc_and_bcc_are_omitted() {
    let recipients = Recipients {
        to: "a@b.c",
        cc: Some("  "),
        bcc: None,
    };
    let raw = build_raw_mime(&recipients, "s", "b", None, &[]).unwrap();
    assert!(!raw.contains("Cc:"));
    assert!(!raw.contains("Bcc:"));
}

#[test]
fn recipient_header_injection_is_rejected() {
    let recipients = Recipients {
        to: "a@b.c",
        cc: None,
        bcc: Some("x@y.z\r\nX-Evil: 1"),
    };
    assert!(build_raw_mime(&recipients, "s", "b", None, &[]).is_err());
}

#[test]
fn summary_lists_recipient_headers() {
    let msg: Message = serde_json::from_value(serde_json::json!({
        "id": "m1",
        "payload": {"headers": [
            {"name": "From", "value": "me@x.y"},
            {"name": "To", "value": "david@hey.com"},
            {"name": "Bcc", "value": "bflora@ycombinator.com"}
        ]}
    }))
    .unwrap();
    let summary = format_message_summary(&msg);
    assert!(summary.contains("To: david@hey.com"));
    assert!(summary.contains("Bcc: bflora@ycombinator.com"));
    assert!(!summary.contains("Cc: \n"));
}
