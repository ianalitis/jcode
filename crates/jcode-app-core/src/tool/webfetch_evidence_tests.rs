//! Synthetic, credential-free fixtures. The loopback response path tests capture
//! and rendering, not live public TLS/DNS. Production admission is tested separately.
use super::*;
#[cfg(unix)]
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn params(value: Value) -> WebFetchInput {
    serde_json::from_value(value).unwrap()
}

fn context() -> ToolContext {
    ToolContext {
        session_id: "evidence-test".into(),
        message_id: "test".into(),
        tool_call_id: "test".into(),
        working_dir: None,
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: super::super::ToolExecutionMode::Direct,
    }
}

async fn serve(wire: Vec<u8>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            // Any request bytes suffice for this fixed, connection-closing response.
            assert!(socket.read(&mut request).await.unwrap() > 0);
            // Oversize tests deliberately stop reading before the server finishes.
            let _ = socket.write_all(&wire).await;
        })
        .await
        .unwrap();
    });
    (format!("http://{addr}/document"), server)
}

async fn fixture(wire: Vec<u8>) -> (reqwest::Response, tokio::task::JoinHandle<()>) {
    let (url, server) = serve(wire).await;
    let response = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .unwrap();
    (response, server)
}

async fn response(body: &[u8], mime: &str) -> (reqwest::Response, tokio::task::JoinHandle<()>) {
    let mut wire = format!("HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
    wire.extend_from_slice(body);
    fixture(wire).await
}

#[cfg(unix)]
async fn capture(root: &Path, body: &[u8], mime: &str) -> ToolOutput {
    prepare_evidence(root).unwrap();
    let (response, server) = response(body, mime).await;
    let input = params(json!({"url":"https://example.org/document", "retain_evidence":true}));
    let output = render_response(response, &input, "evidence-test", Some(root))
        .await
        .unwrap();
    server.await.unwrap();
    output
}

#[cfg(unix)]
fn reference(output: &ToolOutput) -> String {
    output.metadata.as_ref().unwrap()["evidence"]["reference"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[cfg(unix)]
fn entry(root: &Path, output: &ToolOutput) -> std::path::PathBuf {
    root.join(reference(output).strip_prefix("evidence:").unwrap())
}

#[test]
fn receipt_hash_matches_independent_sha256_vector() {
    assert_eq!(
        digest(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn public_url_admission_rejects_unsafe_and_ambiguous_targets() {
    for url in [
        "http://example.org/",
        "https://example.org:444/",
        "https://user@example.org/",
        "https://user:pass@example.org/",
        "https://example.org/?q=x",
        "https://example.org/#part",
        "https://localhost/",
        "https://sub.localhost/",
        "https://127.1/",
        "https://2130706433/",
        "https://0x7f000001/",
        "https://10.0.0.1/",
        "https://[::1]/",
        "https://[::ffff:127.0.0.1]/",
    ] {
        assert!(research_url(url).is_err(), "admitted {url}");
    }
    assert!(research_url("https://example.org/docs/").is_ok());
    assert!(research_url("https://[2606:4700:4700::1111]/").is_ok());
    assert!(research_url(&format!("https://example.org/{}", "a".repeat(4096))).is_err());
}

#[test]
fn public_address_policy_rejects_special_use_ranges() {
    for ip in [
        "0.1.2.3",
        "10.2.3.4",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.1.1",
        "192.0.0.8",
        "192.0.2.1",
        "192.88.99.1",
        "198.18.0.1",
        "198.19.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "::ffff:8.8.8.8",
        "64:ff9b::808:808",
        "fc00::1",
        "fe80::1",
        "ff02::1",
        "2001::1",
        "2001:2::1",
        "2001:db8::1",
        "2002::1",
        "3fff::1",
    ] {
        assert!(!public_address(ip.parse().unwrap()), "admitted {ip}");
    }
    for ip in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
        assert!(public_address(ip.parse().unwrap()));
    }
}

#[test]
fn pinned_client_rejects_empty_mixed_private_and_wrong_port_answers() {
    assert!(pinned_research_client("example.org", &[]).is_err());
    for answers in [
        vec!["1.1.1.1:443", "127.0.0.1:443"],
        vec!["10.0.0.1:443"],
        vec!["1.1.1.1:80"],
    ] {
        let answers: Vec<_> = answers.iter().map(|a| a.parse().unwrap()).collect();
        assert!(pinned_research_client("example.org", &answers).is_err());
    }
    assert!(pinned_research_client("example.org", &["1.1.1.1:443".parse().unwrap()]).is_ok());
}

#[tokio::test]
async fn public_execute_denies_capture_before_local_network() {
    let tool = WebFetchTool::new();
    let mut anonymous = context();
    anonymous.session_id.clear();
    assert!(
        tool.execute(
            json!({"url":"https://example.org/", "retain_evidence":true}),
            anonymous
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("requires a session")
    );
    for url in [
        "http://127.0.0.1:1/",
        "https://127.0.0.1/",
        "https://example.org/?secret=x",
    ] {
        let error = tool
            .execute(json!({"url":url,"retain_evidence":true}), context())
            .await
            .unwrap_err();
        assert!(error.to_string().starts_with("Research capture"));
    }
    assert!(
        tool.execute(json!({"url":"evidence:../../private"}), context())
            .await
            .is_err()
    );
    assert!(
        tool.execute(
            json!({"url":"evidence:fetch-abcdefghijklmnop", "retain_evidence":true}),
            context()
        )
        .await
        .is_err()
    );
    assert!(
        tool.execute(json!({"url":"https://example.org/", "offset":0}), context())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn default_mode_retains_original_rendering_without_evidence() {
    assert!(!params(json!({"url":"https://example.org/"})).retain_evidence);
    let (response, server) = response(b"<h1>Hello</h1><p>world</p>", "text/html").await;
    let input = params(json!({"url":"https://example.org/"}));
    let output = render_response(response, &input, "test", None)
        .await
        .unwrap();
    server.await.unwrap();
    assert!(output.output.contains("# Hello"));
    assert!(output.output.starts_with("Fetched https://example.org/"));
    assert!(output.metadata.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn restarted_tool_rereads_without_refetch_and_default_fetch_does_not_write() {
    const CHILD_REF: &str = "JCODE_WEBFETCH_RESTART_TEST_REFERENCE";
    if let Ok(reference) = std::env::var(CHILD_REF) {
        // This exact test is re-entered in a fresh process with an empty environment
        // plus the temporary home and opaque reference explicitly supplied below.
        let tool = WebFetchTool::new();
        let output = tool
            .execute(json!({"url":reference}), context())
            .await
            .unwrap();
        assert!(output.output.ends_with("restart snapshot"));
        let root = evidence_root().unwrap();
        let before = std::fs::read_dir(&root).unwrap().count();
        // `execute` refuses loopback destinations by design, so drive the
        // default (non-retaining) render path directly with a real response.
        let (response, server) = response(b"default fetch", "text/plain").await;
        let input = params(json!({"url":"https://example.org/document"}));
        let output = render_response(response, &input, "evidence-test", None)
            .await
            .unwrap();
        server.await.unwrap();
        assert!(output.output.ends_with("default fetch"));
        assert!(output.metadata.is_none());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), before);
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("research-evidence");
    let output = capture(&root, b"restart snapshot", "text/plain").await;
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "tool::webfetch::evidence_tests::restarted_tool_rereads_without_refetch_and_default_fetch_does_not_write", "--test-threads=1"])
        .env_clear()
        .env("HOME", temp.path()).env("JCODE_HOME", temp.path())
        .env("JCODE_NO_TELEMETRY", "1").env("DO_NOT_TRACK", "1")
        .env(CHILD_REF, reference(&output))
        .output().unwrap();
    assert!(
        status.status.success(),
        "isolated restart test failed:\n{}\n{}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
}

#[cfg(unix)]
#[tokio::test]
async fn fetch_once_rereads_original_snapshot_after_server_stops() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    let body = format!("<h1>Original</h1><p>{}</p>", "source line\n".repeat(5000));
    let first = capture(&root, body.as_bytes(), "text/html").await;
    let metadata = first.metadata.as_ref().unwrap();
    let receipt = &metadata["evidence"];
    assert_eq!(receipt["raw_sha256"], digest(body.as_bytes()));
    assert_ne!(receipt["raw_sha256"], receipt["text_sha256"]);
    assert_eq!(metadata["excerpt_truncated"], true);
    assert_eq!(metadata["next_offset"], 8000);
    assert_eq!(metadata["craap"], "not_assessed");
    assert_eq!(receipt["body_complete"], true);
    assert_eq!(receipt["lossy_utf8"], false);
    let input = params(json!({"url":reference(&first), "offset":8000,"limit":40000}));
    let reread = read_evidence(
        &root,
        &input,
        "evidence-test",
        chrono::Utc::now().timestamp(),
    )
    .unwrap();
    assert_eq!(reread.metadata.as_ref().unwrap()["evidence"], *receipt);
    assert!(reread.output.contains("source line"));
    // A new observation never overwrites an earlier one, even for the same URL.
    let second = capture(&root, b"changed", "text/plain").await;
    assert_ne!(reference(&first), reference(&second));
    assert_ne!(
        receipt["raw_sha256"],
        second.metadata.as_ref().unwrap()["evidence"]["raw_sha256"]
    );
    let original = read_evidence(
        &root,
        &params(json!({"url":reference(&first)})),
        "evidence-test",
        chrono::Utc::now().timestamp(),
    )
    .unwrap();
    assert!(original.output.contains("# Original"));
    assert_eq!(
        std::fs::read(entry(&root, &first).join("raw")).unwrap(),
        body.as_bytes()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn session_expiry_reference_and_utf8_bounds_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    let output = capture(&root, "aé🦀z".as_bytes(), "text/plain").await;
    let url = reference(&output);
    let now = chrono::Utc::now().timestamp();
    let input = params(json!({"url":url}));
    assert!(read_evidence(&root, &input, "", now).is_err());
    assert!(read_evidence(&root, &input, "another-session", now).is_err());
    let expires = output.metadata.as_ref().unwrap()["evidence"]["expires_at"]
        .as_i64()
        .unwrap();
    assert!(read_evidence(&root, &input, "evidence-test", expires).is_err());
    assert!(read_evidence(&root, &input, "evidence-test", expires - 1).is_ok());
    assert!(
        entry(&root, &output).exists(),
        "expiry must not silently delete files"
    );
    for (offset, limit) in [(2, 8), (1, 1), (3, 3), (99, 8), (0, 0), (0, 40001)] {
        assert!(
            read_evidence(
                &root,
                &params(json!({"url":url,"offset":offset,"limit":limit})),
                "evidence-test",
                now
            )
            .is_err()
        );
    }
    let page = read_evidence(
        &root,
        &params(json!({"url":url,"offset":1,"limit":2})),
        "evidence-test",
        now,
    )
    .unwrap();
    assert!(page.output.ends_with("é"));
    assert_eq!(page.metadata.unwrap()["next_offset"], 3);
    assert!(
        read_evidence(
            &root,
            &params(json!({"url":url,"offset":8})),
            "evidence-test",
            now
        )
        .is_ok()
    );
    for bad in [
        "evidence:../raw",
        "evidence:fetch-abcdefghijklmnop/../raw",
        "evidence:fetch-abcdefghijklmnop",
    ] {
        assert!(read_evidence(&root, &params(json!({"url":bad})), "evidence-test", now).is_err());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn modified_or_oversize_snapshot_is_not_returned() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    for file in ["raw", "text", "receipt.json"] {
        let output = capture(&root, b"original", "text/plain").await;
        let bytes = if file == "receipt.json" {
            vec![b' '; 16_385]
        } else {
            b"modified".to_vec()
        };
        std::fs::write(entry(&root, &output).join(file), bytes).unwrap();
        assert!(
            read_evidence(
                &root,
                &params(json!({"url":reference(&output)})),
                "evidence-test",
                chrono::Utc::now().timestamp()
            )
            .is_err()
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn store_rejects_symlinks_hardlinks_and_broad_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    let output = capture(&root, b"private", "text/plain").await;
    let dir = entry(&root, &output);
    let input = params(json!({"url":reference(&output)}));
    let now = chrono::Utc::now().timestamp();
    assert_eq!(
        std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(dir.join("raw"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    std::fs::set_permissions(dir.join("raw"), std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(read_evidence(&root, &input, "evidence-test", now).is_err());
    std::fs::set_permissions(dir.join("raw"), std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::hard_link(dir.join("raw"), temp.path().join("hardlink")).unwrap();
    assert!(read_evidence(&root, &input, "evidence-test", now).is_err());
    let output = capture(&root, b"another", "text/plain").await;
    let dir = entry(&root, &output);
    std::fs::rename(dir.join("raw"), dir.join("raw-original")).unwrap();
    symlink(dir.join("raw-original"), dir.join("raw")).unwrap();
    assert!(
        read_evidence(
            &root,
            &params(json!({"url":reference(&output)})),
            "evidence-test",
            now
        )
        .is_err()
    );
    symlink(&root, temp.path().join("symlink-root")).unwrap();
    assert!(prepare_evidence(&temp.path().join("symlink-root")).is_err());
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(prepare_evidence(&root).is_err());
}

#[cfg(unix)]
#[test]
fn quota_busy_lock_and_invalid_root_fail_before_capture() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    prepare_evidence(&root).unwrap();
    let lock = evidence_lock(&root).unwrap();
    assert!(prepare_evidence(&root).is_err());
    drop(lock);
    for i in 0..MAX_EVIDENCE_RECORDS {
        std::fs::create_dir(root.join(format!("incomplete-{i}"))).unwrap();
    }
    assert!(
        prepare_evidence(&root)
            .unwrap_err()
            .to_string()
            .contains("full")
    );
    std::fs::write(temp.path().join("not-directory"), b"untouched").unwrap();
    assert!(prepare_evidence(&temp.path().join("not-directory")).is_err());
    assert_eq!(
        std::fs::read(temp.path().join("not-directory")).unwrap(),
        b"untouched"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn incomplete_stream_prefix_and_lossy_utf8_are_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    let output = capture(&root, &[b'a', 0xff, b'z'], "text/plain").await;
    assert_eq!(output.metadata.unwrap()["evidence"]["lossy_utf8"], true);
    let mut wire =
        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n".to_vec();
    wire.extend(vec![b'x'; MAX_SIZE + 1]);
    let (response, server) = fixture(wire).await;
    let input = params(json!({"url":"https://example.org/","retain_evidence":true}));
    let output = render_response(response, &input, "evidence-test", Some(&root))
        .await
        .unwrap();
    server.await.unwrap();
    let metadata = output.metadata.unwrap();
    assert_eq!(metadata["evidence"]["body_complete"], false);
    assert_eq!(metadata["evidence"]["raw_bytes"], MAX_SIZE);
    assert_eq!(metadata["evidence"]["text_bytes"], MAX_SIZE);
    assert_eq!(metadata["excerpt_truncated"], true);
}

#[cfg(unix)]
#[tokio::test]
async fn response_errors_publish_no_reference_or_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    prepare_evidence(&root).unwrap();
    for wire in [
        b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 0\r\n\r\n"
            .to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 1\r\n\r\nx".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 100\r\n\r\nshort".to_vec(),
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
            MAX_SIZE + 1
        )
        .into_bytes(),
    ] {
        let (response, server) = fixture(wire).await;
        let input = params(json!({"url":"https://example.org/","retain_evidence":true}));
        assert!(
            render_response(response, &input, "evidence-test", Some(&root))
                .await
                .is_err()
        );
        server.await.unwrap();
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            1,
            "only the lock remains"
        );
    }
}

// ---------------------------------------------------------------------------
// The store is a 24-hour cache, and reads already refuse an expired record. Until
// now nothing removed them, so dead snapshots held the 32 slots forever and the
// store stayed full until a human cleared it.
// ---------------------------------------------------------------------------

fn write_record(root: &std::path::Path, name: &str, expires_at: i64) {
    let dir = root.join(name);
    std::fs::create_dir(&dir).expect("record dir");
    let receipt = EvidenceReceipt {
        version: 1,
        reference: format!("evidence:{name}"),
        session_hash: "test-session".to_string(),
        requested_url: "https://example.org/document".to_string(),
        final_url: "https://example.org/document".to_string(),
        fetched_at: "2026-09-22T00:00:00+00:00".to_string(),
        expires_at,
        status: 200,
        content_type: "text/plain".to_string(),
        transform: "webfetch-v1:text".to_string(),
        raw_sha256: "0".repeat(64),
        text_sha256: "0".repeat(64),
        raw_bytes: 1,
        text_bytes: 1,
        body_complete: true,
        lossy_utf8: false,
    };
    std::fs::write(
        dir.join("receipt.json"),
        serde_json::to_vec(&receipt).expect("receipt serializes"),
    )
    .expect("receipt written");
}

#[cfg(unix)]
#[test]
fn an_expired_snapshot_makes_room_instead_of_blocking_retention() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    prepare_evidence(&root).unwrap();
    let now = chrono::Utc::now().timestamp();

    for index in 0..MAX_EVIDENCE_RECORDS - 1 {
        write_record(&root, &format!("live-{index}"), now + 3_600);
    }
    write_record(&root, "expired-0", now - 60);

    prepare_evidence(&root).expect("the expired record frees a slot");

    assert!(
        !root.join("expired-0").exists(),
        "the expired snapshot is gone"
    );
    assert!(root.join("live-0").exists(), "live snapshots are untouched");
}

#[cfg(unix)]
#[test]
fn a_store_of_live_snapshots_still_refuses_and_says_why() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    prepare_evidence(&root).unwrap();
    let now = chrono::Utc::now().timestamp();

    for index in 0..MAX_EVIDENCE_RECORDS {
        write_record(&root, &format!("live-{index}"), now + 3_600);
    }

    let error = prepare_evidence(&root).expect_err("a full store of live records still refuses");
    let message = error.to_string();
    assert!(message.contains("full"), "{message}");
    assert!(
        message.contains("none expired"),
        "the message must not blame expiry it did not find: {message}"
    );
}

#[cfg(unix)]
#[test]
fn a_capture_without_a_receipt_is_never_pruned() {
    // A record with no receipt is mid-capture or malformed. Neither is evidence that
    // the quota is held by something already unreadable, so it must keep counting.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("evidence");
    prepare_evidence(&root).unwrap();

    for index in 0..MAX_EVIDENCE_RECORDS {
        std::fs::create_dir(root.join(format!("incomplete-{index}"))).unwrap();
    }

    let error = prepare_evidence(&root).expect_err("unfinished captures hold the quota");
    assert!(error.to_string().contains("full"), "{error}");
    assert!(root.join("incomplete-0").exists());
}
