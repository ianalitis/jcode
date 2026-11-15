use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct KeepAliveFixture {
    base: String,
    connections: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for KeepAliveFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn keep_alive_fixture() -> KeepAliveFixture {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let connections = Arc::new(AtomicUsize::new(0));
    let count = connections.clone();
    let task = tokio::spawn(async move {
        let mut clients = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (mut stream, _) = accepted.unwrap();
                    stream.set_nodelay(true).unwrap();
                    count.fetch_add(1, Ordering::SeqCst);
                    clients.spawn(async move {
                        // Controlled connection setup cost, not a WAN measurement.
                        tokio::time::sleep(Duration::from_millis(30)).await;
                        let mut buffered = Vec::new();
                        loop {
                            let header_end = loop {
                                if let Some(end) = buffered.windows(4).position(|part| part == b"\r\n\r\n") {
                                    break end + 4;
                                }
                                let mut chunk = [0; 1024];
                                let Ok(n) = stream.read(&mut chunk).await else { return; };
                                if n == 0 { return; }
                                buffered.extend_from_slice(&chunk[..n]);
                                assert!(buffered.len() <= 1024 * 1024);
                            };
                            let headers = String::from_utf8_lossy(&buffered[..header_end]);
                            let length = headers.lines().find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().unwrap())
                            }).unwrap_or(0);
                            while buffered.len() < header_end + length {
                                let mut chunk = [0; 1024];
                                let Ok(n) = stream.read(&mut chunk).await else { return; };
                                if n == 0 { return; }
                                buffered.extend_from_slice(&chunk[..n]);
                            }
                            buffered.drain(..header_end + length);
                            if stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: keep-alive\r\n\r\n").await.is_err() {
                                return;
                            }
                            // Ensure success headers and body arrive separately.
                            tokio::time::sleep(Duration::from_millis(8)).await;
                            if stream.write_all(b"{\"success\":true}").await.is_err() {
                                return;
                            }
                        }
                    });
                }
                Some(result) = clients.join_next(), if !clients.is_empty() => { result.unwrap(); }
            }
        }
    });
    KeepAliveFixture {
        base,
        connections,
        task,
    }
}

fn body() -> UploadBody<'static> {
    UploadBody {
        title: None,
        source: "local",
        host: None,
        cwd: None,
        model: None,
        created_at: String::new(),
        updated_at: String::new(),
        messages: &[],
    }
}

#[tokio::test]
#[ignore = "controlled HTTP keep-alive benchmark with injected connection/body delays"]
async fn benchmark_mobile_upload_keep_alive() {
    for _ in 0..3 {
        let fixture = keep_alive_fixture().await;
        let client = reqwest::Client::new();
        let started = Instant::now();
        for _ in 0..8 {
            upload(&client, &fixture.base, "fixture", "test", &body())
                .await
                .unwrap();
        }
        eprintln!(
            "upload keep-alive fixture: uploads=8 duration_ms={:.3} connections={}",
            started.elapsed().as_secs_f64() * 1000.0,
            fixture.connections.load(Ordering::SeqCst)
        );
    }
}

#[tokio::test]
async fn mobile_upload_reuses_connection_after_separately_arriving_success_body() {
    let fixture = keep_alive_fixture().await;
    let client = reqwest::Client::new();
    for _ in 0..4 {
        upload(&client, &fixture.base, "fixture", "test", &body())
            .await
            .unwrap();
    }
    assert_eq!(fixture.connections.load(Ordering::SeqCst), 1);
}

async fn body_fixture(
    headers: &str,
    bytes: Vec<u8>,
    keep_open: bool,
) -> (String, tokio::task::JoinHandle<Vec<u8>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let response = format!("HTTP/1.1 200 OK\r\n{headers}\r\n\r\n");
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();
        let mut request = Vec::new();
        let header_end = loop {
            if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                break end + 4;
            }
            let mut chunk = [0; 1024];
            let count = stream.read(&mut chunk).await.unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&chunk[..count]);
        };
        let length = String::from_utf8_lossy(&request[..header_end])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        while request.len() < header_end + length {
            let mut chunk = [0; 1024];
            let count = stream.read(&mut chunk).await.unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&chunk[..count]);
        }
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.write_all(&bytes).await.unwrap();
        if keep_open {
            // Much longer than the drain budget. The caller aborts this task.
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
        request
    });
    (base, task)
}

#[tokio::test]
async fn sync_redacts_exports_without_changing_sources_or_incremental_progress() {
    let _lock = storage::lock_test_env();
    let home = tempfile::tempdir().unwrap();
    let _env = super::tests::TestEnv::set(&[
        ("JCODE_HOME", home.path().as_os_str()),
        ("JCODE_API_KEY", std::ffi::OsStr::new("fixture-key")),
    ]);
    DIRTY_SESSIONS.lock().unwrap().clear();
    save_state(&MobileSyncState {
        enabled: true,
        ..Default::default()
    })
    .unwrap();
    let token = format!("sk-proj-{}", "a".repeat(48));
    let mut session = Session::create_with_id("redacted-upload".into(), None, None);
    session.title = Some(format!("Title {token}"));
    session.custom_title = Some(format!("Custom {token}"));
    session.add_message(
        Role::User,
        vec![ContentBlock::Text {
            text: format!("Visible {token}"),
            cache_control: None,
        }],
    );
    session.add_message(
        Role::Assistant,
        vec![ContentBlock::ToolUse {
            id: "tool".into(),
            name: "bash".into(),
            input: serde_json::json!({"command": format!("echo {token}")}),
            thought_signature: None,
        }],
    );
    session.add_message(
        Role::User,
        vec![ContentBlock::ToolResult {
            tool_use_id: "tool".into(),
            content: format!("Result {token}"),
            is_error: None,
        }],
    );
    let client = reqwest::Client::new();
    for (phase, start) in [("initial", 0), ("append", 3), ("rewrite", 0), ("shrink", 0)] {
        match phase {
            "append" => {
                session.add_message(
                    Role::User,
                    vec![ContentBlock::Text {
                        text: format!("Appended {token}"),
                        cache_control: None,
                    }],
                );
                session.custom_title = None;
            }
            "rewrite" => {
                let mut messages = session.messages.clone();
                messages[0].content = vec![ContentBlock::Text {
                    text: format!("Rewritten {token}"),
                    cache_control: None,
                }];
                session.replace_messages(messages);
            }
            "shrink" => session.truncate_messages(1),
            _ => {}
        }
        session.save().unwrap();
        let path = home.path().join("sessions/redacted-upload.json");
        let before = std::fs::read(&path).unwrap();
        let stored = serde_json::to_value(Session::load_from_path(&path).unwrap()).unwrap();
        let (base, server) = body_fixture("Content-Length: 2", b"{}".to_vec(), false).await;
        let _endpoint =
            super::tests::TestEnv::set(&[("JCODE_API_BASE", std::ffi::OsStr::new(&base))]);
        let report = tokio::time::timeout(Duration::from_secs(5), sync_once(&client))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(report.errors, 0);
        assert_eq!(report.sessions_uploaded, 1);
        let request = server.await.unwrap();
        let offset = request
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        let payload: serde_json::Value = serde_json::from_slice(&request[offset..]).unwrap();
        assert!(
            !payload.to_string().contains(&token),
            "unredacted {phase} export"
        );
        let exported = session.redacted_for_export();
        assert_eq!(
            payload["title"],
            serde_json::to_value(exported.custom_title.as_ref().or(exported.title.as_ref()))
                .unwrap()
        );
        let expected = wire_messages(&exported, start);
        assert_eq!(
            payload["messages"],
            serde_json::to_value(&expected).unwrap()
        );
        assert_eq!(report.messages_uploaded, expected.len());
        assert_eq!(
            load_state().sessions[&session.id].uploaded,
            session.messages.len()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(
            serde_json::to_value(Session::load_from_path(&path).unwrap()).unwrap(),
            stored
        );
        assert!(stored.to_string().contains(&token));
        assert_eq!(sync_once(&client).await.unwrap(), SyncReport::default());
    }
    DIRTY_SESSIONS.lock().unwrap().clear();
}

#[tokio::test]
async fn mobile_upload_body_drain_is_time_bounded() {
    let (base, server) = body_fixture("Content-Length: 1", Vec::new(), true).await;
    let response = reqwest::get(&base).await.unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(2), drain_upload_response(response))
        .await
        .unwrap();
    assert_eq!(outcome, "timeout");
    server.abort();
}

#[tokio::test]
async fn mobile_upload_body_drain_bounds_unknown_chunked_body() {
    let payload = vec![b'x'; UPLOAD_RESPONSE_DRAIN_LIMIT + 1];
    let mut chunked = format!("{:x}\r\n", payload.len()).into_bytes();
    chunked.extend(payload);
    chunked.extend_from_slice(b"\r\n");
    // No terminating chunk: without the byte cap this would wait for timeout.
    let (base, server) = body_fixture("Transfer-Encoding: chunked", chunked, true).await;
    let response = reqwest::get(&base).await.unwrap();
    assert_eq!(drain_upload_response(response).await, "size_limit");
    server.abort();
}

#[tokio::test]
async fn mobile_upload_body_drain_preserves_exact_boundary_and_empty_success() {
    for bytes in [Vec::new(), vec![b'x'; UPLOAD_RESPONSE_DRAIN_LIMIT]] {
        let (base, server) =
            body_fixture(&format!("Content-Length: {}", bytes.len()), bytes, false).await;
        let response = reqwest::get(&base).await.unwrap();
        assert_eq!(drain_upload_response(response).await, "success");
        server.await.unwrap();
    }
}

#[tokio::test]
async fn mobile_upload_body_drain_classifies_truncation_without_error_text() {
    let (base, server) = body_fixture("Content-Length: 100", b"PRIVATE".to_vec(), false).await;
    let response = reqwest::get(&base).await.unwrap();
    assert_eq!(drain_upload_response(response).await, "read_error");
    server.await.unwrap();
}

#[tokio::test]
async fn mobile_upload_accepted_status_is_not_retried_for_bad_or_stalled_body() {
    for (headers, bytes, keep_open) in [
        ("Content-Length: 100", b"PRIVATE".to_vec(), false),
        ("Content-Length: 100", Vec::new(), true),
        (
            "Content-Length: 1000000",
            vec![b'x'; UPLOAD_RESPONSE_DRAIN_LIMIT + 1],
            true,
        ),
    ] {
        let (base, server) = body_fixture(headers, bytes, keep_open).await;
        tokio::time::timeout(
            Duration::from_secs(2),
            upload(&reqwest::Client::new(), &base, "fixture", "test", &body()),
        )
        .await
        .unwrap()
        .unwrap();
        server.abort();
    }
}
