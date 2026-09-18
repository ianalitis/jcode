use super::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) enum ServerAction {
    Response {
        status: &'static str,
        headers: String,
        body: String,
    },
    Drop,
    /// Send a 200 with an SSE content type, one delta, then hold the
    /// connection open without finishing for `hold_ms`. Used by deadline and
    /// cancellation tests.
    Stall {
        hold_ms: u64,
    },
}

pub(super) struct TestServer {
    pub(super) api_base: String,
    pub(super) destination: String,
    requests: Arc<AtomicUsize>,
    handle: JoinHandle<()>,
}

impl TestServer {
    pub(super) fn spawn(actions: Vec<ServerAction>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback server");
        listener
            .set_nonblocking(true)
            .expect("set loopback listener nonblocking");
        let address = listener.local_addr().expect("loopback address");
        let requests = Arc::new(AtomicUsize::new(0));
        let request_count = Arc::clone(&requests);
        let handle = std::thread::spawn(move || {
            let started = Instant::now();
            let mut completed_at = actions.is_empty().then(Instant::now);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let index = request_count.fetch_add(1, Ordering::SeqCst);
                        read_request(&mut stream);
                        match actions.get(index).cloned().unwrap_or(ServerAction::Drop) {
                            ServerAction::Response {
                                status,
                                headers,
                                body,
                            } => {
                                let response = format!(
                                    "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
                                    body.len()
                                );
                                let _ = stream.write_all(response.as_bytes());
                                let _ = stream.flush();
                            }
                            ServerAction::Drop => {}
                            ServerAction::Stall { hold_ms } => {
                                let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n";
                                let _ = stream.write_all(head.as_bytes());
                                let _ = stream.flush();
                                std::thread::sleep(Duration::from_millis(hold_ms));
                            }
                        }
                        if index + 1 >= actions.len() {
                            completed_at.get_or_insert_with(Instant::now);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("loopback accept failed: {error}"),
                }

                if completed_at.is_some_and(|at| at.elapsed() >= Duration::from_millis(300))
                    || started.elapsed() >= Duration::from_secs(5)
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let api_base = format!("http://{address}/v1");
        let destination = format!("{api_base}/chat/completions");
        Self {
            api_base,
            destination,
            requests,
            handle,
        }
    }

    pub(super) fn join(self) -> usize {
        let requests = self.requests;
        self.handle.join().expect("join loopback server");
        requests.load(Ordering::SeqCst)
    }
}

fn read_request(stream: &mut std::net::TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set request read timeout");
    let mut request = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                request.extend_from_slice(&buffer[..read]);
                if let Some(header_end) =
                    request.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    let header_end = header_end + 4;
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.split_once(':').and_then(|(name, value)| {
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                        })
                        .unwrap_or(0);
                    if request.len() >= header_end + content_length {
                        break;
                    }
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => panic!("request read failed: {error}"),
        }
    }
}

pub(super) fn response(status: &'static str, body: impl Into<String>) -> ServerAction {
    ServerAction::Response {
        status,
        headers: String::new(),
        body: body.into(),
    }
}

pub(super) fn success_response() -> ServerAction {
    response(
        "200 OK",
        concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        ),
    )
}

pub(super) fn synthetic_provider(api_base: String) -> OpenRouterProvider {
    OpenRouterProvider {
        client: jcode_provider_core::shared_http_client(),
        model: Arc::new(RwLock::new("approved/model".to_string())),
        reasoning_effort: Arc::new(RwLock::new(None)),
        api_base,
        auth: ProviderAuth::None {
            label: "synthetic no-auth fixture".to_string(),
        },
        supports_provider_features: false,
        supports_model_catalog: false,
        profile_id: Some("synthetic".to_string()),
        reasoning_effort_support: Some(false),
        disable_reasoning_heuristics: true,
        static_reasoning_config: HashMap::new(),
        max_tokens: None,
        extra_body: None,
        static_models: Vec::new(),
        static_context_limits: HashMap::new(),
        static_image_input_support: HashMap::new(),
        send_openrouter_headers: false,
        conversation_id: new_conversation_id(),
        models_cache: Arc::new(RwLock::new(ModelsCache::default())),
        model_catalog_refresh: Arc::new(Mutex::new(ModelCatalogRefreshState::default())),
        provider_routing: Arc::new(RwLock::new(ProviderRouting::default())),
        provider_pin: Arc::new(Mutex::new(None)),
        endpoints_cache: Arc::new(RwLock::new(HashMap::new())),
        endpoint_refresh: Arc::new(Mutex::new(EndpointRefreshTracker::default())),
    }
}

pub(super) fn fixture_request(messages: &[Message]) -> Value {
    json!({
        "model": "approved/model",
        "messages": jcode_provider_openrouter::request::build_chat_messages(
            messages, "", false, false, false,
        ),
        "stream": true,
        "stream_options": {"include_usage": true}
    })
}

fn collect_single_send(
    provider: &OpenRouterProvider,
    expected: Value,
    destination: &str,
    messages: &[Message],
) -> Result<Vec<Result<StreamEvent>>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("single-send runtime");
    runtime.block_on(async {
        let mut stream = provider
            .complete_single_send_with_expected_final_request(
                expected,
                destination,
                messages,
                &[],
                "",
                None,
            )
            .await?;
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        Ok(events)
    })
}

#[test]
fn single_send_success_dispatches_exactly_once() {
    let server = TestServer::spawn(vec![success_response()]);
    let provider = synthetic_provider(server.api_base.clone());
    let messages = vec![Message::user("approved prompt")];
    let events = collect_single_send(
        &provider,
        fixture_request(&messages),
        &server.destination,
        &messages,
    )
    .expect("single-send completion starts");
    assert!(events.iter().all(Result::is_ok), "events: {events:?}");
    assert_eq!(server.join(), 1);
}

#[test]
fn single_send_retryable_statuses_dispatch_exactly_once_each() {
    for status in [
        "429 Too Many Requests",
        "500 Internal Server Error",
        "503 Service Unavailable",
    ] {
        let server = TestServer::spawn(vec![response(status, "retryable fixture")]);
        let provider = synthetic_provider(server.api_base.clone());
        let messages = vec![Message::user("approved prompt")];
        let events = collect_single_send(
            &provider,
            fixture_request(&messages),
            &server.destination,
            &messages,
        )
        .expect("single-send completion starts");
        assert!(events.iter().any(Result::is_err), "{status}: {events:?}");
        assert_eq!(server.join(), 1, "{status}");
    }
}

#[test]
fn single_send_redirect_does_not_reach_target() {
    let target = TestServer::spawn(Vec::new());
    let source = TestServer::spawn(vec![ServerAction::Response {
        status: "307 Temporary Redirect",
        headers: format!("Location: {}\r\n", target.destination),
        body: String::new(),
    }]);
    let provider = synthetic_provider(source.api_base.clone());
    let messages = vec![Message::user("approved prompt")];
    let events = collect_single_send(
        &provider,
        fixture_request(&messages),
        &source.destination,
        &messages,
    )
    .expect("single-send completion starts");
    assert!(events.iter().any(Result::is_err), "events: {events:?}");
    assert_eq!(source.join(), 1);
    assert_eq!(target.join(), 0);
}

#[test]
fn single_send_destination_mismatch_dispatches_zero_requests() {
    let server = TestServer::spawn(Vec::new());
    let provider = synthetic_provider(server.api_base.clone());
    let messages = vec![Message::user("approved prompt")];
    let error = collect_single_send(
        &provider,
        fixture_request(&messages),
        &format!("{}/different", server.api_base),
        &messages,
    )
    .expect_err("mismatched destination must fail closed");
    assert!(error.to_string().contains("trusted expected destination"));
    assert_eq!(server.join(), 0);
}

#[test]
fn single_send_rejects_unsafe_destination_components_before_dispatch() {
    let messages = vec![Message::user("approved prompt")];
    for (api_base, expected_error) in [
        ("http://example.invalid/v1", "must use HTTPS"),
        (
            "https://user@example.invalid/v1",
            "forbidden URL components",
        ),
        (
            "https://example.invalid/v1#fragment",
            "forbidden URL components",
        ),
    ] {
        let provider = synthetic_provider(api_base.to_string());
        let destination = format!("{api_base}/chat/completions");
        let error = collect_single_send(
            &provider,
            fixture_request(&messages),
            &destination,
            &messages,
        )
        .expect_err("unsafe destination must fail before dispatch");
        assert!(error.to_string().contains(expected_error), "{error:#}");
    }
}

#[test]
fn single_send_dropped_response_is_not_retried() {
    let server = TestServer::spawn(vec![ServerAction::Drop]);
    let provider = synthetic_provider(server.api_base.clone());
    let messages = vec![Message::user("approved prompt")];
    let events = collect_single_send(
        &provider,
        fixture_request(&messages),
        &server.destination,
        &messages,
    )
    .expect("single-send completion starts");
    assert!(events.iter().any(Result::is_err), "events: {events:?}");
    assert_eq!(server.join(), 1);
}

#[test]
fn ordinary_retry_path_still_retries() {
    let server = TestServer::spawn(vec![
        response("503 Service Unavailable", "retry fixture"),
        success_response(),
    ]);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("ordinary retry runtime");
    runtime.block_on(async {
        let (tx, mut rx) = mpsc::channel::<Result<StreamEvent>>(32);
        super::openrouter_sse_stream::run_stream_with_retries(
            Client::builder()
                .no_proxy()
                .build()
                .expect("ordinary test client"),
            server.api_base.clone(),
            ProviderAuth::None {
                label: "synthetic no-auth fixture".to_string(),
            },
            false,
            new_conversation_id(),
            json!({"model": "approved/model", "messages": [], "stream": true}),
            tx,
            Arc::new(Mutex::new(None)),
            "approved/model".to_string(),
        )
        .await;
        while let Some(event) = rx.recv().await {
            event.expect("ordinary retry path should recover");
        }
    });
    assert_eq!(server.join(), 2);
}

#[test]
fn single_send_final_body_mismatch_dispatches_zero_requests() {
    let server = TestServer::spawn(Vec::new());
    let provider = synthetic_provider(server.api_base.clone());
    let messages = vec![Message::user("approved prompt")];
    let mut expected = fixture_request(&messages);
    expected["model"] = json!("different/model");
    let error = collect_single_send(&provider, expected, &server.destination, &messages)
        .expect_err("final-body mismatch must fail closed");
    assert!(error.to_string().contains("trusted expected request"));
    assert_eq!(server.join(), 0);
}
