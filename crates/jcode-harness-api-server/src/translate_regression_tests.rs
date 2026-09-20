use super::*;

#[test]
fn persisted_metadata_reads_large_transcripts_from_bounded_windows() {
    let home = ScopedJcodeHome::new("bounded-metadata");
    let sessions = home.path.join("sessions");
    std::fs::create_dir_all(&sessions).expect("create sessions directory");
    let path = sessions.join("session_large.json");
    let mut file = std::fs::File::create(&path).expect("create large session");
    write!(
        file,
        "{{\"id\":\"session_large\",\"title\":\"Generated title\",\"messages\":[\""
    )
    .unwrap();
    for _ in 0..(2 * 1024) {
        file.write_all(&[b'x'; 1024]).unwrap();
    }
    write!(
        file,
        "\"],\"working_dir\":\"/workspace/large\",\"custom_title\":\"Pinned title\"}}"
    )
    .unwrap();
    drop(file);

    let metadata = BridgeState::resolve_session_metadata("session_large").expect("metadata");
    assert_eq!(metadata.working_dir.as_deref(), Some("/workspace/large"));
    assert_eq!(metadata.title.as_deref(), Some("Generated title"));
    assert_eq!(metadata.custom_title.as_deref(), Some("Pinned title"));
    assert_eq!(metadata.display_title().as_deref(), Some("Pinned title"));
}

#[test]
fn outbound_layout_does_not_inline_the_large_reply_payload() {
    let outbound = std::mem::size_of::<Outbound>();
    let legacy = std::mem::size_of::<Value>();
    let frame = std::mem::size_of::<ServerFrame>();
    eprintln!("layout bytes: Outbound={outbound}, Legacy={legacy}, ServerFrame={frame}");
    assert!(
        outbound <= 2 * legacy,
        "large replies inflate every legacy action: {outbound} bytes"
    );
}

#[test]
fn local_reply_retains_server_frame_wire_encoding() {
    let out = BridgeState::default().api_request_to_legacy(&json!({"req": "ping", "id": 7}));
    let [Outbound::Reply(frame)] = out.as_slice() else {
        panic!("expected local reply")
    };
    assert_eq!(
        serde_json::to_value(frame).unwrap(),
        serde_json::to_value(ServerFrame::reply(7, ApiEvent::Pong)).unwrap()
    );
}

#[test]
fn metadata_string_keeps_first_last_null_and_missing_semantics() {
    let bytes = br#"{"title":"first","nested":{"title":"last"}}"#;
    assert_eq!(
        BridgeState::metadata_string(bytes, "title", false).as_deref(),
        Some("first")
    );
    assert_eq!(
        BridgeState::metadata_string(bytes, "title", true).as_deref(),
        Some("last")
    );
    assert_eq!(BridgeState::metadata_string(bytes, "missing", true), None);
    let bytes = br#"[{"title":"first"},{"title":null}]"#;
    assert_eq!(BridgeState::metadata_string(bytes, "title", true), None);
}

#[test]
fn stored_sessions_keep_descending_mtime_order_and_limit() {
    let home = ScopedJcodeHome::new("mtime-order");
    for (id, seconds) in [("middle", 20), ("oldest", 10), ("newest", 30)] {
        let path = write_session_record(&home.path, id, &home.path);
        std::fs::File::open(path)
            .unwrap()
            .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(seconds))
            .unwrap();
    }
    assert_eq!(
        BridgeState::stored_session_ids(Some(2)),
        ["newest", "middle"]
    );
    assert_eq!(
        BridgeState::stored_session_ids(None),
        ["newest", "middle", "oldest"]
    );
    assert!(BridgeState::stored_session_ids(Some(0)).is_empty());
}
