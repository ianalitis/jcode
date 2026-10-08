use super::*;

#[test]
fn control_replies_preserve_native_denials_and_single_context_wrapper() {
    for field in [2, 3, 4, 5, 7, 8, 9, 14, 16, 17, 18, 20, 21, 22, 23] {
        let mut request = wire::field_varint(1, 71);
        request.extend(wire::field_str(15, "control-fixture"));
        request.extend(wire::field_ld(field, &[]));
        let message = wire::decode_exec_server_message(&request).unwrap();
        let reply = wire::encode_control_exec_reply(&message, &[]).expect("native denial reply");
        let wrapper: Vec<_> = wire::iter_fields(&reply).collect();
        assert_eq!(wrapper.len(), 1);
        assert_eq!(wrapper[0].field, 2);
        let fields: Vec<_> = wire::iter_fields(wrapper[0].data).collect();
        assert!(fields.iter().any(|f| f.field == 1 && f.varint == 71));
        assert!(
            fields
                .iter()
                .any(|f| f.field == 15 && f.data == b"control-fixture")
        );
        assert!(fields.iter().any(|f| f.field == field));
        assert!(
            !fields.iter().any(|f| f.field == 11),
            "native path must not become MCP success"
        );
        assert!(String::from_utf8_lossy(&reply).contains("Re-issue"));
    }
    let message = wire::ExecServerMessage {
        id: 73,
        exec_id: "context".into(),
        variant: wire::ExecServerMessageVariant::RequestContext(wire::RequestContextArgs {}),
    };
    assert_eq!(
        wire::encode_control_exec_reply(&message, &[]).unwrap(),
        wire::encode_request_context_result(73, "context", &[]),
        "context acknowledgement is empty and must not be double wrapped"
    );
}

#[tokio::test]
async fn agent_reply_delivery_reports_closed_stream_without_payload() {
    let (tx, mut rx) = mpsc::channel(1);
    let payload = b"private-reply-fixture";
    agent_transport::send_agent_reply(&tx, payload)
        .await
        .unwrap();
    assert_eq!(rx.recv().await.unwrap(), wire::connect_frame(payload));
    drop(rx);
    let error = agent_transport::send_agent_reply(&tx, payload)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Cursor agent outbound stream closed before reply"
    );
    assert!(!error.to_string().contains("private-reply-fixture"));
}

#[test]
fn control_allowlist_only_accepts_the_authorized_bridge() {
    for (provider, allowed) in [(wire::JCODE_TOOL_PROVIDER, true), ("untrusted", false)] {
        let message = wire::ExecServerMessage {
            id: 74,
            exec_id: "allowlist".into(),
            variant: wire::ExecServerMessageVariant::McpAllowlistPrecheck {
                provider_identifier: provider.into(),
            },
        };
        assert_eq!(
            wire::encode_control_exec_reply(&message, &[]).unwrap(),
            wire::encode_agent_client_exec_message(&wire::encode_allowlist_precheck_result(
                74,
                "allowlist",
                42,
                allowed,
            ))
        );
    }
    for field in [41, 43] {
        let message = wire::ExecServerMessage {
            id: 75,
            exec_id: "denied".into(),
            variant: wire::ExecServerMessageVariant::OtherAllowlistPrecheck(field),
        };
        assert_eq!(
            wire::encode_control_exec_reply(&message, &[]).unwrap(),
            wire::encode_agent_client_exec_message(&wire::encode_allowlist_precheck_result(
                75, "denied", field, false,
            ))
        );
    }
}
