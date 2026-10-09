// One listener can serve preflight + decision, and captures the exact wire
// requests without touching credentials, process environment, or real APIs.
type MockReply = (u16, String, Vec<(String, String)>);

fn mock_server(replies: Vec<MockReply>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let count = replies.len();
    let mut replies = replies.into_iter();
    mock_server_with(count, move |_| replies.next().unwrap())
}

/// Serve `count` requests, choosing each reply from the request itself, so
/// concurrent requests get correct replies whatever order they connect in.
fn mock_server_with(
    count: usize,
    mut respond: impl FnMut(&str) -> MockReply + Send + 'static,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..count {
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "expected mock request"
                        );
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            };
            // On BSD/macOS an accepted socket inherits the listener's
            // non-blocking flag, so without this the timed read below
            // returns WouldBlock instead of waiting for the request.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let n = stream.read(&mut buffer).unwrap();
                assert_ne!(n, 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let len = headers
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let request = String::from_utf8(bytes).unwrap();
            let (status, body, headers) = respond(&request);
            requests.push(request);
            let chunked = headers.iter().any(|(key, value)| {
                key.eq_ignore_ascii_case("transfer-encoding") && value == "chunked"
            });
            let extra: String = headers
                .into_iter()
                .map(|(k, v)| format!("{k}: {v}\r\n"))
                .collect();
            let (length_header, wire_body) = if chunked {
                (
                    String::new(),
                    format!("{:x}\r\n{body}\r\n0\r\n\r\n", body.len()),
                )
            } else {
                (format!("Content-Length: {}\r\n", body.len()), body)
            };
            let reply = format!(
                "HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\n{length_header}Connection: close\r\n{extra}\r\n{wire_body}"
            );
            // A bounded client may close early on an oversized response.
            let _ = stream.write_all(reply.as_bytes());
        }
        requests
    });
    (base, worker)
}

fn mock_client(base: &str, provider: JevProvider) -> JevClient {
    JevClient {
        client: client_builder().no_proxy().build().unwrap(),
        hedge_clients: VOICE_HEDGE_DELAYS
            .iter()
            .map(|_| client_builder().no_proxy().build().unwrap())
            .collect(),
        purpose: JevPurpose::Memory,
        provider,
        api_key: "test-route-secret".into(),
        endpoint: format!("{base}/v1/decisions"),
        me_endpoint: (provider == JevProvider::Jcode).then(|| format!("{base}/v1/me")),
    }
}

#[tokio::test]
async fn voice_subscription_uses_existing_noul_entitlement_without_bypassing_it() {
    for entitled in [false, true] {
        let mut replies = vec![(
            200,
            json!({"capabilities": {"memory_jev": entitled, "browser_jev": false}}).to_string(),
            vec![],
        )];
        if entitled {
            replies.push((200, response().to_string(), vec![]));
        }
        let (base, worker) = mock_server(replies);
        let mut client = mock_client(&base, JevProvider::Jcode);
        client.purpose = JevPurpose::Voice;
        let result = client
            .evaluate(json!({"transcript": "synthetic"}), questions())
            .await;
        let requests = worker.join().unwrap();
        assert!(requests[0].starts_with("GET /v1/me "));
        if entitled {
            result.unwrap();
            assert_eq!(requests.len(), 2);
            let body: Value =
                serde_json::from_str(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(body["model"], "typesafe/jev-1.13");
            assert!(body["state"].is_string());
            assert!(
                !requests[1]
                    .to_ascii_lowercase()
                    .contains("x-jcode-jev-purpose"),
                "only browser decisions use the browser budget"
            );
        } else {
            let error = result.unwrap_err().to_string();
            assert!(error.contains("voice"));
            assert!(error.contains("memory_jev"));
            assert_eq!(
                requests.len(),
                1,
                "denied voice entitlement must not send a decision"
            );
        }
    }
}

/// Serve every connection on its own thread. Connection `n` waits
/// `delays[n]` before replying, so a slow primary can overlap a hedge.
fn concurrent_mock(
    delays: Vec<(Duration, u16)>,
) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    std::thread::spawn(move || {
        for (index, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else { return };
            let Some(&(delay, status)) = delays.get(index) else {
                return;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                while !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                    let n = stream.read(&mut buffer).unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                }
                std::thread::sleep(delay);
                let body = response().to_string();
                let _ = stream.write_all(format!("HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    (base, seen)
}

#[tokio::test]
async fn voice_hedges_a_slow_request_and_takes_the_first_valid_answer() {
    let (base, seen) = concurrent_mock(vec![(Duration::from_secs(3), 200), (Duration::ZERO, 200)]);
    let mut client = mock_client(&base, JevProvider::TypeSafe);
    client.purpose = JevPurpose::Voice;
    let started = std::time::Instant::now();
    let value = client
        .evaluate(json!({"transcript": "synthetic"}), questions())
        .await
        .unwrap();
    assert!(validate_answers(&value, &questions()).is_ok());
    assert!(started.elapsed() < Duration::from_secs(2), "hedge won");
    assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn voice_hedge_failure_waits_for_primary_and_memory_never_hedges() {
    let (base, seen) = concurrent_mock(vec![
        (Duration::from_millis(300), 200),
        (Duration::ZERO, 500),
    ]);
    let mut client = mock_client(&base, JevProvider::TypeSafe);
    client.purpose = JevPurpose::Voice;
    client
        .evaluate(json!({"transcript": "synthetic"}), questions())
        .await
        .unwrap();
    assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 2);

    let (base, seen) = concurrent_mock(vec![(Duration::from_millis(300), 200)]);
    mock_client(&base, JevProvider::TypeSafe)
        .evaluate(json!({"transcript": "synthetic"}), questions())
        .await
        .unwrap();
    assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn voice_typesafe_errors_never_retry_or_return_a_partial_report() {
    for status in [401, 402, 403, 500, 302] {
        let (base, worker) = mock_server(vec![(
            status,
            "private-error test-route-secret".into(),
            vec![("Location".into(), "http://127.0.0.1:1/never-follow".into())],
        )]);
        let mut client = mock_client(&base, JevProvider::TypeSafe);
        client.purpose = JevPurpose::Voice;
        let error = crate::voice_intent::classify_with_client("Start a new session", &[], &client)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(&status.to_string()));
        assert!(!error.contains("test-route-secret"));
        assert!(!error.contains("private-error"));
        assert_eq!(worker.join().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn voice_direct_transport_preserves_typed_state_and_never_sends_openrouter_headers() {
    use crate::voice_intent::{QuickAction, VoiceIntent, classify_with_client, describe_questions};
    let transcript = "Start a new Jcode conversation";
    let answers: Map<String, Value> = describe_questions(transcript, &[])
        .unwrap()
        .into_iter()
        .map(|question| {
            let probability = match question.id.as_str() {
                "quick_action" | "new_session" => 0.99,
                _ => 0.01,
            };
            (question.id, json!({"type": "noul", "noul": probability}))
        })
        .collect();
    let (base, worker) = mock_server(vec![(200, json!({"answers": answers}).to_string(), vec![])]);
    let mut client = mock_client(&base, JevProvider::TypeSafe);
    client.purpose = JevPurpose::Voice;
    client.endpoint = format!("{base}/v1/systemone");
    let report = classify_with_client(transcript, &[], &client)
        .await
        .unwrap();
    assert_eq!(
        report.intent,
        VoiceIntent::QuickAction(QuickAction::NewSession)
    );
    let requests = worker.join().unwrap();
    assert_eq!(
        requests.len(),
        1,
        "direct API has no gateway entitlement preflight"
    );
    let request = &requests[0];
    assert!(request.starts_with("POST /v1/systemone "));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-route-secret\r\n")
    );
    assert!(!request.to_ascii_lowercase().contains("http-referer:"));
    assert!(!request.to_ascii_lowercase().contains("x-title:"));
    let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["model"], "jev-latest");
    assert_eq!(body["state"]["transcript"], transcript);
    assert_eq!(body["questions"].as_object().unwrap().len(), 6);
}

#[tokio::test]
async fn voice_twenty_candidates_cross_real_transport_in_bounded_same_state_batches() {
    use crate::voice_intent::{
        SessionCandidate, VoiceIntent, classify_with_client, describe_questions,
    };
    let offered: Vec<_> = (0..20)
        .map(|index| SessionCandidate {
            id: format!("private-session-{index}"),
            title: format!("Investigate voice routing request budgets and preserve classification safety for recent conversation {index}: compare candidate metadata, JSON escaping, provider transport limits, and regression coverage"),
            working_dir: Some(r#"C:\Users\example\projects\jcode\"voice routing""#.into()),
        })
        .collect();
    let transcript = "Open conversation 19";
    let questions = describe_questions(transcript, &offered).unwrap();
    assert_eq!(questions.len(), 26);
    // Exercise both object-state and string-state providers, including the
    // gateway's real entitlement preflight on every bounded evaluation.
    for provider in [
        JevProvider::TypeSafe,
        JevProvider::OpenRouter,
        JevProvider::Aimlapi,
        JevProvider::Jcode,
    ] {
        // All batches participate in ranking, with no competing-score veto.
        for competing in [0.01, 0.9, 0.99, 1.0] {
            let batches = questions.chunks(provider.max_questions()).count();
            let per_batch = if provider == JevProvider::Jcode { 2 } else { 1 };
            // Batches run concurrently, so answer whatever each request asks.
            let (base, worker) = mock_server_with(batches * per_batch, move |request| {
                if request.starts_with("GET ") {
                    let me = json!({"capabilities": {"memory_jev": true}});
                    return (200, me.to_string(), vec![]);
                }
                let body: Value =
                    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
                let answers: Map<String, Value> = body["questions"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(|id| {
                        let probability = match id.as_str() {
                            "navigation" | "candidate_19" => 0.99,
                            "coding_agent" => competing,
                            _ => 0.01,
                        };
                        (id.clone(), json!({"type": "noul", "noul": probability}))
                    })
                    .collect();
                (200, json!({"answers": answers}).to_string(), vec![])
            });
            let result = classify_with_client(transcript, &offered, &mock_client(&base, provider))
                .await
                .unwrap();
            assert_eq!(
                result.intent,
                if competing < 0.99 {
                    VoiceIntent::OpenSession(offered[19].id.clone())
                } else {
                    VoiceIntent::CodingAgent
                }
            );
            assert_eq!(result.answers.len(), 26);
            assert_eq!(
                result.answers.iter().map(|a| &a.id).collect::<Vec<_>>(),
                questions.iter().map(|q| &q.id).collect::<Vec<_>>()
            );
            assert_eq!(
                result
                    .answers
                    .iter()
                    .find(|a| a.id == "candidate_19")
                    .unwrap()
                    .probability,
                0.99
            );
            let requests = worker.join().unwrap();
            let bodies: Vec<Value> = requests
                .iter()
                .filter(|r| r.starts_with("POST "))
                .map(|r| {
                    let wire = r.split_once("\r\n\r\n").unwrap().1;
                    assert!(wire.len() <= MAX_REQUEST_BYTES);
                    assert!(wire.len() < 32 * 1024, "voice batch bytes: {}", wire.len());
                    serde_json::from_str(wire).unwrap()
                })
                .collect();
            assert_eq!(bodies.len(), batches);
            assert!(
                bodies
                    .iter()
                    .all(|body| body["state"] == bodies[0]["state"])
            );
            let state = if let Some(text) = bodies[0]["state"].as_str() {
                serde_json::from_str::<Value>(text).unwrap()
            } else {
                bodies[0]["state"].clone()
            };
            assert_eq!(state["candidates"].as_object().unwrap().len(), 20);
            assert_eq!(state["transcript"], transcript);
            assert!(
                state["policy"]
                    .as_str()
                    .unwrap()
                    .contains("untrusted evidence")
            );
            for (index, candidate) in offered.iter().enumerate() {
                assert_eq!(
                    state["candidates"][format!("candidate_{index}")]["title"],
                    candidate.title
                );
                assert_eq!(
                    state["candidates"][format!("candidate_{index}")]["working_dir"],
                    candidate.working_dir.as_deref().unwrap()
                );
            }
            assert!(!state.to_string().contains("private-session"));
            let mut sent = Map::new();
            for body in &bodies {
                let batch = body["questions"].as_object().unwrap();
                assert!(batch.len() <= provider.max_questions());
                sent.extend(batch.clone());
            }
            assert_eq!(sent.len(), questions.len());
            for question in &questions {
                assert_eq!(sent[&question.id]["instructions"], question.instructions);
                assert_eq!(sent[&question.id]["criteria"]["true"], question.yes);
                assert_eq!(sent[&question.id]["criteria"]["false"], question.no);
            }
        }
    }
}

#[tokio::test]
async fn voice_later_batch_errors_never_return_partial_classification() {
    use crate::voice_intent::{SessionCandidate, classify_with_client, describe_questions};
    let offered: Vec<_> = (0..20)
        .map(|index| SessionCandidate {
            id: format!("session-{index}"),
            title: format!("Conversation {index}"),
            working_dir: None,
        })
        .collect();
    let questions = describe_questions("next conversation", &offered).unwrap();
    let first: Map<String, Value> = questions[..MAX_QUESTIONS]
        .iter()
        .map(|q| {
            (
                q.id.clone(),
                json!({"type": "noul", "noul": if q.id == "coding_agent" { 0.99 } else { 0.01 }}),
            )
        })
        .collect();
    let last: Map<String, Value> = questions[MAX_QUESTIONS..]
        .iter()
        .map(|q| (q.id.clone(), json!({"type": "noul", "noul": 0.01})))
        .collect();
    let mut failures = vec![
        (500, "{}".into(), vec![]),
        (200, json!({"answers": {}}).to_string(), vec![]),
    ];
    for invalid in [
        json!({"type": "noul", "noul": 1.1}),
        json!({"type": "noul", "noul": "0.99"}),
        json!({"type": "choice", "choice": "yes"}),
    ] {
        let mut bad = last.clone();
        bad.insert(questions[MAX_QUESTIONS].id.clone(), invalid);
        failures.push((200, json!({"answers": bad}).to_string(), vec![]));
    }
    let mut wrong_ids = last.clone();
    wrong_ids.remove(&questions[MAX_QUESTIONS].id);
    wrong_ids.insert("invented".into(), json!({"type": "noul", "noul": 0.99}));
    failures.push((200, json!({"answers": wrong_ids}).to_string(), vec![]));
    for failure in failures {
        // Batches run concurrently, so reply by which batch was requested.
        let later = questions[MAX_QUESTIONS].id.clone();
        let good = json!({"answers": first}).to_string();
        let mut failure = Some(failure);
        let (base, worker) = mock_server_with(2, move |request| {
            let body: Value =
                serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
            if body["questions"].get(&later).is_some() {
                failure.take().unwrap()
            } else {
                (200, good.clone(), vec![])
            }
        });
        assert!(
            classify_with_client(
                "next conversation",
                &offered,
                &mock_client(&base, JevProvider::OpenRouter)
            )
            .await
            .is_err()
        );
        assert_eq!(worker.join().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn browser_subscription_checks_browser_capability_and_posts_choice() {
    let answer =
        json!({"answers": {"action": {"type": "choice", "choice": "click", "confidence": 0.9}}});
    let (base, worker) = mock_server(vec![
        (
            200,
            json!({"capabilities": {"browser_jev": true, "memory_jev": false}}).to_string(),
            vec![],
        ),
        (200, answer.to_string(), vec![]),
    ]);
    let mut client = mock_client(&base, JevProvider::Jcode);
    client.purpose = JevPurpose::Browser;
    assert_eq!(client.provider_name(), "jcode");
    assert_eq!(client.model_id(), "typesafe/jev-1.13");
    assert_eq!(
        client
            .evaluate(json!({"page": "private-page"}), browser_questions())
            .await
            .unwrap(),
        answer
    );
    let requests = worker.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("GET /v1/me "));
    assert!(!requests[0].contains("private-page"));
    assert!(requests[1].starts_with("POST /v1/decisions "));
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains("x-jcode-jev-purpose: browser\r\n"),
        "browser decisions must use the gateway's separate browser budget"
    );
    let body: Value = serde_json::from_str(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["questions"], Value::Object(browser_questions()));
    assert!(body["state"].is_string());
}

#[tokio::test]
async fn browser_capability_denial_prevents_page_upload() {
    for me in [
        json!({"capabilities": {"memory_jev": true}}),
        json!({"capabilities": {"browser_jev": false}}),
        json!({"capabilities": {"browser_jev": "true"}}),
    ] {
        let (base, worker) = mock_server(vec![(200, me.to_string(), vec![])]);
        let mut client = mock_client(&base, JevProvider::Jcode);
        client.purpose = JevPurpose::Browser;
        let error = client
            .evaluate(json!("private-page"), browser_questions())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("browser_jev"));
        let requests = worker.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /v1/me "));
        assert!(!requests[0].contains("private-page"));
    }
}

#[tokio::test]
async fn browser_auth_billing_and_redirect_failures_never_retry_or_fallback() {
    for provider in [JevProvider::Jcode, JevProvider::OpenRouter] {
        for preflight in [false, true] {
            if preflight && provider != JevProvider::Jcode {
                continue;
            }
            for status in [401, 402, 403, 302, 307] {
                let mut replies = Vec::new();
                if provider == JevProvider::Jcode && !preflight {
                    replies.push((
                        200,
                        json!({"capabilities": {"browser_jev": true}}).to_string(),
                        vec![],
                    ));
                }
                replies.push((
                    status,
                    "private-provider-error test-route-secret".into(),
                    vec![("Location".into(), "http://127.0.0.1:1/never-follow".into())],
                ));
                let expected_requests = replies.len();
                let (base, worker) = mock_server(replies);
                let mut client = mock_client(&base, provider);
                client.purpose = JevPurpose::Browser;
                let error = client
                    .evaluate(json!("private-page"), browser_questions())
                    .await
                    .unwrap_err();
                let detail = format!("{error:#}");
                assert!(detail.contains(&status.to_string()));
                for secret in [
                    "private-provider-error",
                    "test-route-secret",
                    "private-page",
                ] {
                    assert!(!detail.contains(secret));
                }
                let requests = worker.join().unwrap();
                assert_eq!(requests.len(), expected_requests);
                if preflight {
                    assert!(!requests[0].contains("private-page"));
                }
            }
        }
    }
}

#[tokio::test]
async fn subscription_requires_live_capability_then_sends_bound_bearer() {
    let (base, worker) = mock_server(vec![
        (
            200,
            json!({"capabilities": {"memory_jev": true}}).to_string(),
            vec![],
        ),
        (200, response().to_string(), vec![]),
    ]);
    let client = mock_client(&base, JevProvider::Jcode);
    let value = client.evaluate(json!("state"), questions()).await.unwrap();
    assert_eq!(value, response());
    let requests = worker.join().unwrap();
    assert!(requests[0].starts_with("GET /v1/me "));
    assert!(requests[1].starts_with("POST /v1/decisions "));
    for request in requests {
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-route-secret\r\n")
        );
        assert!(!request.lines().next().unwrap().contains("secret"));
    }
}

#[tokio::test]
async fn missing_or_false_capability_never_posts_decisions() {
    for me in [
        json!({"tier": "flagship", "status": "active"}),
        json!({"capabilities": {"memory_jev": false}}),
        json!({"capabilities": {"memory_jev": "true"}}),
    ] {
        let (base, worker) = mock_server(vec![(200, me.to_string(), vec![])]);
        let client = mock_client(&base, JevProvider::Jcode);
        let error = client
            .evaluate(json!("state"), questions())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("memory_jev"));
        assert_eq!(worker.join().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn transient_overload_is_retried_on_same_route_then_succeeds() {
    for status in [429, 502, 503, 504, 529] {
        let (base, worker) = mock_server(vec![
            (
                status,
                "{}".into(),
                vec![("Retry-After".into(), "1".into())],
            ),
            (status, "{}".into(), vec![]),
            (200, response().to_string(), vec![]),
        ]);
        let client = mock_client(&base, JevProvider::OpenRouter);
        let value = client.evaluate(json!("state"), questions()).await.unwrap();
        assert_eq!(value, response());
        let requests = worker.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(
            requests
                .iter()
                .all(|r| r.starts_with("POST /v1/decisions "))
        );
    }
}

#[tokio::test]
async fn gateway_plan_quota_is_not_retried_and_becomes_upgrade_prompt() {
    let quota = json!({"error": {
        "code": "quota_exceeded", "message": "server text", "purpose": "browser", "scope": "day",
        "limit": 2000, "tier": "plus", "upgrade_tier": "pro",
        "upgrade_url": "https://jcode.sh/pricing", "resets_at": "2026-09-27T00:00:00.000Z"
    }});
    let (base, worker) = mock_server(vec![
        (
            200,
            json!({"capabilities": {"browser_jev": true}}).to_string(),
            vec![],
        ),
        (
            429,
            quota.to_string(),
            vec![("Retry-After".into(), "80000".into())],
        ),
    ]);
    let mut client = mock_client(&base, JevProvider::Jcode);
    client.purpose = JevPurpose::Browser;
    let error = client
        .evaluate(json!({"page": "p"}), browser_questions())
        .await
        .unwrap_err();
    let notice = crate::subscription_notice::from_error(&error).expect("typed quota notice");
    assert_eq!(notice.feature, "browser");
    assert_eq!(notice.upgrade_tier.as_deref(), Some("pro"));
    assert!(error.to_string().contains("Upgrade to Pro"));
    assert!(error.to_string().contains("https://jcode.sh/pricing"));
    // One /me preflight and exactly one decision: plan quotas are never retried.
    assert_eq!(worker.join().unwrap().len(), 2);
}

#[tokio::test]
async fn gateway_quota_rejects_non_jcode_upgrade_links() {
    let quota = json!({"error": {"code": "quota_exceeded", "purpose": "memory", "tier": "plus",
        "upgrade_tier": "pro", "upgrade_url": "https://evil.example/pay"}});
    let (base, worker) = mock_server(vec![
        (
            200,
            json!({"capabilities": {"memory_jev": true}}).to_string(),
            vec![],
        ),
        (
            429,
            quota.to_string(),
            vec![("Retry-After".into(), "80000".into())],
        ),
    ]);
    let client = mock_client(&base, JevProvider::Jcode);
    let error = client.evaluate(json!("s"), questions()).await.unwrap_err();
    let notice = crate::subscription_notice::from_error(&error).unwrap();
    assert_eq!(notice.upgrade_url, None);
    assert!(!error.to_string().contains("evil.example"));
    worker.join().unwrap();
}

#[tokio::test]
async fn persistent_overload_fails_after_bounded_retries_without_echo() {
    for status in [429, 529] {
        let replies = (0..=TRANSIENT_RETRY_DELAYS.len())
            .map(|_| {
                (
                    status,
                    "private-provider-error test-route-secret".into(),
                    vec![],
                )
            })
            .collect();
        let (base, worker) = mock_server(replies);
        let client = mock_client(&base, JevProvider::OpenRouter);
        let detail = format!(
            "{:#}",
            client
                .evaluate(json!("private-state"), questions())
                .await
                .unwrap_err()
        );
        assert!(detail.contains(&status.to_string()));
        assert!(detail.contains("overloaded"));
        assert!(!detail.contains("test-route-secret"));
        assert!(!detail.contains("private-state"));
        assert_eq!(
            worker.join().unwrap().len(),
            TRANSIENT_RETRY_DELAYS.len() + 1
        );
    }
}

#[tokio::test]
async fn opencode_free_names_the_workspace_setting_that_blocks_it() {
    // Zen reports the free-endpoint refusal as a plain 400, so the operator has
    // to be told which setting to change; the paid route must not carry it.
    let (base, worker) = mock_server(vec![(400, "{}".into(), vec![])]);
    let client = mock_client(&base, JevProvider::OpenCodeFree);
    let error = client
        .evaluate(json!("state"), questions())
        .await
        .unwrap_err();
    let detail = format!("{error:#}");
    assert!(detail.contains("400"));
    assert!(detail.contains("workspace privacy setting"));
    assert!(detail.contains("training on request data"));
    assert_eq!(worker.join().unwrap().len(), 1);

    let (base, worker) = mock_server(vec![(400, "{}".into(), vec![])]);
    let client = mock_client(&base, JevProvider::OpenCode);
    let error = client
        .evaluate(json!("state"), questions())
        .await
        .unwrap_err();
    let detail = format!("{error:#}");
    assert!(detail.contains("model unavailable"));
    assert!(!detail.contains("workspace privacy setting"));
    assert_eq!(worker.join().unwrap().len(), 1);
}

#[tokio::test]
async fn auth_billing_and_redirect_errors_are_redacted_and_never_retried() {
    for status in [401, 402, 403, 404, 500, 302, 307] {
        let headers = vec![("Location".into(), "http://127.0.0.1:1/never-follow".into())];
        let (base, worker) = mock_server(vec![(
            status,
            "private-provider-error test-route-secret".into(),
            headers,
        )]);
        let client = mock_client(&base, JevProvider::OpenRouter);
        let error = client
            .evaluate(json!("private-state"), questions())
            .await
            .unwrap_err();
        let detail = format!("{error:#}");
        assert!(detail.contains(&status.to_string()));
        assert!(!detail.contains("test-route-secret"));
        assert!(!detail.contains("private-provider-error"));
        assert!(!detail.contains("private-state"));
        assert_eq!(worker.join().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn oversized_and_invalid_json_responses_are_rejected_without_echo() {
    for body in [
        "x".repeat(MAX_RESPONSE_BYTES + 1),
        "private-response-invalid-json".into(),
    ] {
        let (base, worker) = mock_server(vec![(200, body, vec![])]);
        let client = mock_client(&base, JevProvider::TypeSafe);
        let error = client
            .evaluate(json!("state"), questions())
            .await
            .unwrap_err();
        assert!(!format!("{error:#}").contains("private-response"));
        worker.join().unwrap();
    }
}

#[tokio::test]
async fn streamed_body_limit_is_enforced_without_content_length() {
    let (base, worker) = mock_server(vec![(
        200,
        "x".repeat(MAX_RESPONSE_BYTES + 1),
        vec![("Transfer-Encoding".into(), "chunked".into())],
    )]);
    let client = mock_client(&base, JevProvider::Aimlapi);
    let error = client
        .evaluate(json!("state"), questions())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("bounded response size"));
    worker.join().unwrap();
}

#[tokio::test]
async fn failed_preflight_does_not_send_decision_request() {
    for (status, body, headers) in [
        (401, "private-account-response".into(), vec![]),
        (200, "x".repeat(MAX_ME_BYTES + 1), vec![]),
        (
            307,
            "private-account-response".into(),
            vec![("Location".into(), "https://untrusted.example/me".into())],
        ),
    ] {
        let (base, worker) = mock_server(vec![(status, body, headers)]);
        let client = mock_client(&base, JevProvider::Jcode);
        let error = client
            .evaluate(json!("state"), questions())
            .await
            .unwrap_err();
        assert!(!format!("{error:#}").contains("private-account-response"));
        let requests = worker.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /v1/me "));
    }
}
