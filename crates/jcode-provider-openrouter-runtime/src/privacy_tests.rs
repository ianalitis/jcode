//! Local-only request boundary tests. Sensitive values exist only at runtime.
use super::*;

fn sensitive_card() -> String {
    // A conventional, Luhn-valid test number, assembled rather than embedded.
    ["4111", "1111", "1111", "1111"].join("")
}

fn capture_server(
    status: u16,
    response_body: String,
) -> (std::net::SocketAddr, mpsc::Receiver<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let header_end = loop {
            let mut chunk = [0; 1024];
            let count = socket.read(&mut chunk).unwrap();
            assert!(count > 0, "request ended before headers");
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                break end + 4;
            }
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let length: usize = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse().unwrap())
            })
            .unwrap();
        while bytes.len() < header_end + length {
            let mut chunk = [0; 1024];
            let count = socket.read(&mut chunk).unwrap();
            assert!(count > 0, "request ended before body");
            bytes.extend_from_slice(&chunk[..count]);
        }
        let value: Value = serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
        tx.send(value).unwrap();
        let response = format!(
            "HTTP/1.1 {status} Test\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        socket.write_all(response.as_bytes()).unwrap();
    });
    (address, rx)
}

#[tokio::test]
async fn privacy_complete_sanitizes_merged_body_for_deepseek_and_opencode_go() {
    let _lock = ENV_LOCK.lock();
    for (profile, override_messages) in [("deepseek", false), ("opencode-go", true)] {
        let card = sensitive_card();
        let (address, captured) = capture_server(200, "data: [DONE]\n\n".into());
        let mut extra = serde_json::json!({"extension": {"nested": [card]}, "max_tokens": 4096});
        if override_messages {
            extra["messages"] = serde_json::json!([{"role": "user", "content": card}]);
        }
        let provider = OpenRouterProvider {
            client: Client::builder().no_proxy().build().unwrap(),
            api_base: format!("http://{address}/v1"),
            profile_id: Some(profile.into()),
            supports_model_catalog: false,
            model: Arc::new(RwLock::new("deepseek/privacy-test".into())),
            extra_body: extra.as_object().cloned(),
            ..make_custom_compatible_provider()
        };
        let messages = vec![Message {
            role: Role::Assistant,
            content: vec![
                ContentBlock::Text {
                    text: card.clone(),
                    cache_control: None,
                },
                ContentBlock::Reasoning { text: card.clone() },
                ContentBlock::ToolUse {
                    id: "call-test".into(),
                    name: "test".into(),
                    input: serde_json::json!({"account": card, "password": "opaque"}),
                    thought_signature: None,
                },
            ],
            timestamp: None,
            tool_duration_ms: None,
        }];
        let tools = vec![ToolDefinition {
            defer_loading: false,
            name: "test".into(),
            description: card.clone(),
            input_schema: serde_json::json!({"type": "object", "properties": {"account": {"type": "string", "description": card}}}),
        }];
        let mut stream = provider
            .complete(&messages, &tools, &card, None)
            .await
            .unwrap();
        while let Some(event) = stream.next().await {
            assert!(event.is_ok(), "local stream failed");
        }
        let body = captured.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            !body.to_string().contains(&card),
            "request boundary disclosed a card"
        );
        assert!(
            !body.to_string().contains("opaque"),
            "request boundary disclosed a credential"
        );
        assert!(body["extension"]["nested"][0] == "[REDACTED_CARD]");
        assert_eq!(body["max_tokens"], 4096);
        assert!(
            messages[0]
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::Text { text, .. } if text == &card)),
            "local source was changed"
        );
    }
}

#[tokio::test]
async fn privacy_transport_sanitizes_direct_call_and_withholds_error_body() {
    for (azure_responses, responses_api) in [(false, false), (true, false), (false, true)] {
        let card = sensitive_card();
        let echoed = format!(
            "content_filter {} {}",
            card,
            ["private", "response"].join("-")
        );
        let (address, captured) = capture_server(403, echoed.clone());
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        super::super::openrouter_sse_stream::run_stream_with_retries(
            Client::builder().no_proxy().build().unwrap(),
            format!("http://{address}/v1"),
            ProviderAuth::None {
                label: card.clone(),
            },
            false,
            "privacy-test".into(),
            serde_json::json!({"messages": [{"content": card}], "extensions": [card]}),
            azure_responses,
            tx,
            Arc::new(Mutex::new(None)),
            card.clone(),
            responses_api,
        )
        .await;
        let body = captured.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!body.to_string().contains(&card));
        let mut saw_error = false;
        while let Some(event) = rx.recv().await {
            if let Err(error) = event {
                let text = format!("{error:#}");
                assert!(!text.contains(&card) && !text.contains(&echoed));
                assert!(text.contains("status: 403") && text.contains("content filter"));
                saw_error = true;
            }
        }
        assert!(saw_error);
    }
}

#[tokio::test]
async fn privacy_successful_http_sse_errors_withhold_provider_payloads() {
    for (azure_responses, responses_api) in [(true, false), (false, false), (false, true)] {
        let card = sensitive_card();
        let marker = ["synthetic", "private", "sse", "payload"].join("-");
        let echoed = format!("content_filter status: 403 {card} {marker}");
        let payload = serde_json::json!({
            "type": "error",
            "message": echoed,
            "code": "content_filter",
            "error": {
                "message": echoed,
                "type": "invalid_request_error",
                "code": "content_filter"
            }
        });
        let (address, captured) = capture_server(
            200,
            format!("event: error\ndata: {payload}\n\ndata: [DONE]\n\n"),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        super::super::openrouter_sse_stream::run_stream_with_retries(
            Client::builder().no_proxy().build().unwrap(),
            format!("http://{address}/v1"),
            ProviderAuth::None {
                label: "local fixture".into(),
            },
            false,
            "privacy-test".into(),
            serde_json::json!({"messages": [{"role": "user", "content": "hello"}]}),
            azure_responses,
            tx,
            Arc::new(Mutex::new(None)),
            "privacy-test".into(),
            responses_api,
        )
        .await;
        assert!(captured.recv_timeout(Duration::from_secs(5)).is_ok());
        let mut saw_error = false;
        while let Some(event) = rx.recv().await {
            let rendered = format!("{event:?}");
            assert!(
                !rendered.contains(&card) && !rendered.contains(&marker),
                "SSE event disclosed synthetic sensitive data"
            );
            let text = match event {
                Err(error) => format!("{error:#}"),
                Ok(StreamEvent::Error { message, .. }) => message,
                _ => continue,
            };
            assert!(
                !text.contains(&card) && !text.contains(&marker),
                "SSE error chain disclosed synthetic sensitive data"
            );
            assert!(
                text.contains("details withheld"),
                "missing safe error metadata"
            );
            assert!(text.contains("content filter"), "missing refusal hint");
            assert!(text.contains("status: 403"), "missing status metadata");
            saw_error = true;
        }
        assert!(saw_error, "fixture did not produce an SSE error");
    }
}

#[tokio::test]
async fn privacy_sanitizer_failure_prevents_auth_and_network() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut nested = Value::String(sensitive_card());
    for _ in 0..70 {
        nested = serde_json::json!([nested]);
    }
    let provider = OpenRouterProvider {
        api_base: format!("http://{}/v1", listener.local_addr().unwrap()),
        supports_model_catalog: false,
        extra_body: serde_json::json!({"extension": nested})
            .as_object()
            .cloned(),
        auth: Arc::new(|| panic!("sanitizer failure reached authentication")),
        ..make_custom_compatible_provider()
    };
    let result = provider.complete(&[], &[], "ordinary", None).await;
    assert!(result.is_err());
    if let Err(error) = result {
        assert_eq!(
            error.to_string(),
            "outbound sanitizer failed; content withheld"
        );
    }
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
}
