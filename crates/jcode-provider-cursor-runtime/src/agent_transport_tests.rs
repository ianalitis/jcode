use super::*;

#[test]
fn usable_model_decoder_reads_wire_ids_and_deduplicates() {
    let response = ["composer-2.5", "gpt-5.4-high", "composer-2.5"]
        .into_iter()
        .flat_map(|model| field_ld(1, &field_str(1, model)))
        .collect::<Vec<_>>();
    assert_eq!(
        crate::decode_agent_models(&response).unwrap(),
        vec!["composer-2.5", "gpt-5.4-high"]
    );
}

/// Issue #1226 and the Cursor support guidance: the AgentService accepts
/// only base ids, with speed/effort carried as separate params.
#[test]
fn resolve_model_id_reduces_composite_ids_to_base_ids() {
    let cases = [
        // Cursor rejects bare `grok-4.6` (ERROR_BAD_MODEL_NAME); its
        // catalog only serves the `cursor-grok-4.6-*` ids.
        (
            "cursor-grok-4.6-high-fast",
            "cursor-grok-4.6-high-fast",
            true,
        ),
        ("grok-4.6", "cursor-grok-4.6-high", false),
        ("grok-4.7", "grok-4.7", false),
        ("claude-opus-5-thinking-high", "claude-opus-5", false),
        ("claude-opus-5-thinking-high-fast", "claude-opus-5", true),
        ("gpt-5.4-high", "gpt-5.4", false),
        ("gpt-5.6-sol-xhigh", "gpt-5.6-sol", false),
        ("gemini-3.7-flash-high", "gemini-3.7-flash", false),
        ("gemini-3.1-pro", "gemini-3.1-pro", false),
        ("sonnet-4.6", "sonnet-4.6", false),
        ("cursor:grok-4.6", "cursor-grok-4.6-high", false),
    ];
    for (input, id, fast) in cases {
        assert_eq!(
            resolve_model_id(input, None),
            ResolvedModel {
                id: id.to_string(),
                fast
            },
            "{input}"
        );
    }
}

/// Cursor: `composer-2.5` with no params defaults to the Fast variant.
/// Live catalog (GetUsableModels, 2026-09-29): exact ids pass through,
/// base ids map to the variant Cursor labels with the plain model name.
#[test]
fn resolve_model_id_uses_live_catalog_ids() {
    let catalog: Vec<String> = [
        "composer-2.5",
        "composer-2.5-fast",
        "grok-4.7-high",
        "grok-4.7-high-fast",
        "grok-4.7-low",
        "cursor-grok-4.6-low",
        "cursor-grok-4.6-high",
        "cursor-grok-4.6-high-fast",
        "cursor-grok-4.6-xhigh",
        "claude-opus-5-5-low",
        "claude-opus-5-5-high",
        "claude-opus-5-5-high-fast",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let r = |m: &str, f: Option<bool>| resolve_model_id_with_catalog(m, f, &catalog);

    // Exact ids are sent verbatim.
    assert_eq!(r("cursor-grok-4.6-xhigh", None).id, "cursor-grok-4.6-xhigh");
    assert_eq!(r("grok-4.7-low", None).id, "grok-4.7-low");
    // Bare base ids Cursor rejects map onto the catalog.
    assert_eq!(r("grok-4.6", None).id, "cursor-grok-4.6-high");
    assert_eq!(r("grok-4.6", Some(true)).id, "cursor-grok-4.6-high-fast");
    assert_eq!(r("cursor:grok-4.6", None).id, "cursor-grok-4.6-high");
    assert_eq!(r("claude-opus-5-5", None).id, "claude-opus-5-5-high");
    assert_eq!(r("grok-4.7", None).id, "grok-4.7-high");
    // /fast toggles between catalog siblings.
    let fast = r("cursor-grok-4.6-high", Some(true));
    assert_eq!(
        (fast.id.as_str(), fast.fast),
        ("cursor-grok-4.6-high-fast", true)
    );
    let slow = r("cursor-grok-4.6-high-fast", Some(false));
    assert_eq!(
        (slow.id.as_str(), slow.fast),
        ("cursor-grok-4.6-high", false)
    );
    // Composer keeps its Fast default.
    let composer = r("composer-2.5", None);
    assert_eq!(
        (composer.id.as_str(), composer.fast),
        ("composer-2.5", true)
    );
    // Unknown models fall back to the base id.
    assert_eq!(r("gpt-5.4-high", None).id, "gpt-5.4");
}

#[test]
fn composer_defaults_to_fast_and_respects_override() {
    assert!(resolve_model_id("composer-2.5", None).fast);
    assert_eq!(
        resolve_model_id("composer-2.5-fast", None).id,
        "composer-2.5"
    );
    assert_eq!(resolve_model_id("composer-2-fast", None).id, "composer-2");
    assert!(!resolve_model_id("composer-2.5", Some(false)).fast);
    assert!(resolve_model_id("grok-4.6", Some(true)).fast);
    assert!(!resolve_model_id("grok-4.6-high-fast", Some(false)).fast);
}

#[test]
fn run_request_encodes_base_id_and_fast_param() {
    let frame = build_run_frames(
        "hi",
        &resolve_model_id("cursor-grok-4.6-high-fast", None),
        "/tmp",
        &[],
        "req",
    )
    .into_iter()
    .next()
    .unwrap();
    let (_, payload, _) = next_frame(&frame).unwrap().unwrap();
    let run_request = iter_fields(&payload)
        .find(|field| field.field == 1 && field.wire == 2)
        .unwrap();
    let requested_model = iter_fields(run_request.data)
        .find(|field| field.field == 9 && field.wire == 2)
        .unwrap();
    let id = iter_fields(requested_model.data)
        .find(|field| field.field == 1 && field.wire == 2)
        .and_then(|field| std::str::from_utf8(field.data).ok());
    assert_eq!(id, Some("cursor-grok-4.6-high-fast"));
    let fast = iter_fields(requested_model.data)
        .find(|field| field.field == 3 && field.wire == 2)
        .and_then(|meta| {
            iter_fields(meta.data)
                .find(|field| field.field == 2 && field.wire == 2)
                .and_then(|field| std::str::from_utf8(field.data).ok())
        });
    assert_eq!(fast, Some("true"));
}

#[test]
fn run_request_uses_current_model_metadata_shape() {
    let frame = build_run_frames(
        "hello",
        &resolve_model_id("composer-2.5", Some(false)),
        "/tmp",
        &[],
        "request-id",
    )
    .into_iter()
    .next()
    .unwrap();
    let (_, payload, _) = next_frame(&frame).unwrap().unwrap();
    let run_request = iter_fields(&payload)
        .find(|field| field.field == 1 && field.wire == 2)
        .unwrap();

    let requested_model = iter_fields(run_request.data)
        .find(|field| field.field == 9 && field.wire == 2)
        .unwrap();
    assert_eq!(
        iter_fields(requested_model.data)
            .find(|field| field.field == 1 && field.wire == 2)
            .and_then(|field| std::str::from_utf8(field.data).ok()),
        Some("composer-2.5")
    );
    let fast_metadata = iter_fields(requested_model.data)
        .find(|field| field.field == 3 && field.wire == 2)
        .unwrap();
    assert_eq!(
        iter_fields(fast_metadata.data)
            .find(|field| field.field == 1 && field.wire == 2)
            .and_then(|field| std::str::from_utf8(field.data).ok()),
        Some("fast")
    );
    assert_eq!(
        iter_fields(fast_metadata.data)
            .find(|field| field.field == 2 && field.wire == 2)
            .and_then(|field| std::str::from_utf8(field.data).ok()),
        Some("false")
    );
    assert!(iter_fields(run_request.data).any(|field| field.field == 3 && field.wire == 2));

    let catalog_entries = iter_fields(run_request.data)
        .filter(|field| field.field == 14 && field.wire == 2)
        .collect::<Vec<_>>();
    let catalog_ids = catalog_entries
        .iter()
        .map(|field| {
            iter_fields(field.data)
                .find(|nested| nested.field == 1 && nested.wire == 2)
                .and_then(|nested| std::str::from_utf8(nested.data).ok())
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(catalog_ids, vec!["default", "composer-2.5"]);
    for entry in catalog_entries {
        let metadata = iter_fields(entry.data)
            .find(|field| field.field == 3 && field.wire == 2)
            .unwrap();
        assert_eq!(
            iter_fields(metadata.data)
                .find(|field| field.field == 1 && field.wire == 2)
                .and_then(|field| std::str::from_utf8(field.data).ok()),
            Some("fast")
        );
    }
}

/// Regression test for issue #637: teams routed to a region reject the
/// hardcoded `global` agent host, so a regional endpoint expressed as a full
/// URL must reduce to a bare host.
#[test]
fn normalize_agent_host_reduces_urls_to_bare_hosts() {
    assert_eq!(
        normalize_agent_host("https://agentn.us.api5.cursor.sh"),
        Some("agentn.us.api5.cursor.sh".to_string())
    );
    assert_eq!(
        normalize_agent_host("http://agentn.eu.api5.cursor.sh/agent.v1.AgentService/Run"),
        Some("agentn.eu.api5.cursor.sh".to_string())
    );
    // Already-bare hosts pass through, case-normalized.
    assert_eq!(
        normalize_agent_host("  AgentN.US.api5.cursor.sh  "),
        Some("agentn.us.api5.cursor.sh".to_string())
    );
    assert_eq!(normalize_agent_host(""), None);
    assert_eq!(normalize_agent_host("   "), None);
    assert_eq!(normalize_agent_host("https://"), None);
}

/// The normalized host is used as both the DNS name and the TLS
/// `ServerName`, so an explicit port must be stripped rather than carried
/// into either. Raised in review of the #637 fix.
#[test]
fn normalize_agent_host_strips_explicit_ports() {
    assert_eq!(
        normalize_agent_host("https://agentn.us.api5.cursor.sh:443/agent.v1.AgentService/Run"),
        Some("agentn.us.api5.cursor.sh".to_string())
    );
    assert_eq!(
        normalize_agent_host("agentn.us.api5.cursor.sh:443"),
        Some("agentn.us.api5.cursor.sh".to_string())
    );
    // A bare port with no host is not a usable host.
    assert_eq!(normalize_agent_host(":443"), None);
}

/// `agent_host()` must normalize env overrides the same way it normalizes
/// the cached CLI value: people copy these out of `cli-config.json` or a
/// browser, so the full-URL spelling has to work. Raised in review of #637.
#[test]
fn agent_host_normalizes_env_overrides() {
    // Serialized against other env-mutating tests in this module by the
    // shared lock in jcode-base.
    let _guard = jcode_base::storage::lock_test_env();
    let prev = std::env::var_os("JCODE_CURSOR_AGENT_HOST");

    jcode_base::env::set_var(
        "JCODE_CURSOR_AGENT_HOST",
        "https://agentn.us.api5.cursor.sh/agent.v1.AgentService/Run",
    );
    assert_eq!(agent_host(), "agentn.us.api5.cursor.sh");

    jcode_base::env::set_var("JCODE_CURSOR_AGENT_HOST", "agentn.eu.api5.cursor.sh:443");
    assert_eq!(agent_host(), "agentn.eu.api5.cursor.sh");

    // A blank override must not win; it falls through to the next source.
    jcode_base::env::set_var("JCODE_CURSOR_AGENT_HOST", "   ");
    assert_ne!(agent_host(), "   ");

    match prev {
        Some(value) => jcode_base::env::set_var("JCODE_CURSOR_AGENT_HOST", value),
        None => jcode_base::env::remove_var("JCODE_CURSOR_AGENT_HOST"),
    }
}

#[test]
fn frames_are_well_formed_connect_frames() {
    let frames = build_run_frames(
        "hi",
        &resolve_model_id("composer-2.5", None),
        "/tmp",
        &[],
        "req",
    );
    assert!(!frames.is_empty());
    for frame in &frames {
        assert!(frame.len() >= 5);
        let len = u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]) as usize;
        assert_eq!(
            len + 5,
            frame.len(),
            "frame length prefix must match payload"
        );
        assert_eq!(frame[0], 0, "request frames are uncompressed data frames");
    }
}

#[test]
fn frame0_contains_prompt_and_model() {
    let frames = build_run_frames(
        "PROMPT_MARKER",
        &resolve_model_id("composer-2.5", None),
        "/tmp",
        &[],
        "req",
    );
    let frame0 = &frames[0];
    let hay = String::from_utf8_lossy(frame0);
    assert!(hay.contains("PROMPT_MARKER"));
    assert!(hay.contains("composer-2.5"));
}

#[test]
fn frame0_advertises_mcp_tools() {
    let tool = jcode_message_types::ToolDefinition {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        input_schema: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        defer_loading: false,
    };
    let frames = build_run_frames(
        "hi",
        &resolve_model_id("composer-2.5", None),
        "/tmp",
        &[tool],
        "req",
    );
    let frame0 = &frames[0];
    let hay = String::from_utf8_lossy(frame0);
    assert!(hay.contains("read_file"));
    assert!(hay.contains("Read a file"));
}

#[test]
fn routed_prompt_uses_the_same_collision_safe_aliases_as_wire_tools() {
    let tools = vec![
        jcode_message_types::ToolDefinition {
            name: "mcp__server-a__tool.name".to_string(),
            description: "First".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            defer_loading: false,
        },
        jcode_message_types::ToolDefinition {
            name: "mcp__server_a__tool_name".to_string(),
            description: "Second".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            defer_loading: false,
        },
    ];
    let prompt = routed_prompt("Do the task", "", &tools);
    let aliases = crate::wire::mcp_wire_aliases(&tools);
    for tool in &tools {
        assert!(prompt.contains(aliases.get(&tool.name).unwrap()));
    }
    assert!(prompt.contains("__"));
    assert!(!prompt.contains("mcp__server-a__tool.name"));
}

#[test]
fn extract_answer_text_reads_nested_chunk() {
    // f1 { f1 { f1: "AUTH" } }
    let leaf = field_str(1, "AUTH");
    let mid = field_ld(1, &leaf);
    let top = field_ld(1, &mid);
    assert_eq!(extract_answer_text(&top).as_deref(), Some("AUTH"));
}

#[test]
fn extract_answer_text_ignores_reasoning() {
    // f1 { f4 { f1: "thinking" } } should not be surfaced as answer text.
    let leaf = field_str(1, "thinking");
    let f4 = field_ld(4, &leaf);
    let top = field_ld(1, &f4);
    assert_eq!(extract_answer_text(&top), None);
    assert_eq!(extract_thinking_text(&top).as_deref(), Some("thinking"));
}

#[test]
fn next_frame_parses_uncompressed() {
    let payload = field_str(1, "hello");
    let frame = connect_frame(&payload);
    let (flag, out, consumed) = next_frame(&frame).unwrap().unwrap();
    assert_eq!(flag, 0);
    assert_eq!(consumed, frame.len());
    assert_eq!(out, payload);
}

#[test]
fn heartbeat_is_stable() {
    assert_eq!(heartbeat_frame(), vec![0, 0, 0, 0, 2, 0x3a, 0x00]);
}

#[test]
fn text_extractors_ignore_mixed_wire_types_at_every_level() {
    for branch in [1, 4] {
        // Fixed-width data deliberately looks like nested protobuf text. The
        // shared reader exposes these bytes, unlike the removed local reader.
        let mut leaf = field_varint(1, 150);
        leaf.extend([0x09, b'f', b'a', b'k', b'e', b'6', b'4', b'!', b'!']);
        leaf.extend([0x0d, b'f', b'a', b'k', b'e']);
        leaf.extend(field_str(1, ""));
        leaf.extend(field_ld(1, &[0xff]));
        let mut middle = field_varint(branch, 150);
        middle.extend([
            (branch << 3 | 1) as u8,
            0x0a,
            6,
            b'f',
            b'a',
            b'k',
            b'e',
            b'6',
            b'4',
        ]);
        middle.extend([(branch << 3 | 5) as u8, 0x0a, 2, b'n', b'o']);
        let mut payload = field_varint(1, 150);
        payload.extend([
            0x09,
            (branch << 3 | 2) as u8,
            6,
            0x0a,
            4,
            b'f',
            b'a',
            b'k',
            b'e',
        ]);
        payload.extend([0x0d, (branch << 3 | 2) as u8, 2, 0x0a, 0]);
        let mut noise = payload.clone();
        let mut nested_noise = middle.clone();
        nested_noise.extend(field_ld(branch, &leaf));
        noise.extend(field_ld(1, &nested_noise));
        assert_eq!(extract_answer_text(&noise), None);
        assert_eq!(extract_thinking_text(&noise), None);

        leaf.extend(field_str(1, "real text"));
        middle.extend(field_ld(branch, &leaf));
        payload.extend(field_ld(1, &middle));
        let plain = field_ld(1, &field_ld(branch, &field_str(1, "real text")));
        let answer = (branch == 1).then_some("real text");
        let thinking = (branch == 4).then_some("real text");
        assert_eq!(extract_answer_text(&plain).as_deref(), answer);
        assert_eq!(extract_answer_text(&payload).as_deref(), answer);
        assert_eq!(extract_thinking_text(&plain).as_deref(), thinking);
        assert_eq!(extract_thinking_text(&payload).as_deref(), thinking);
    }
}

#[derive(Default)]
struct TaskState {
    started: std::sync::atomic::AtomicUsize,
    finished: std::sync::atomic::AtomicUsize,
    destroyed: std::sync::atomic::AtomicUsize,
}

#[derive(Clone, Default)]
pub(super) struct TaskProbes(Arc<[TaskState; 2]>);

pub(super) struct TaskGuard {
    probes: TaskProbes,
    index: usize,
    started: bool,
}

impl TaskProbes {
    pub(super) fn guard(&self, index: usize) -> TaskGuard {
        TaskGuard {
            probes: self.clone(),
            index,
            started: false,
        }
    }

    fn counts(&self) -> [(usize, usize, usize); 2] {
        use std::sync::atomic::Ordering::SeqCst;
        std::array::from_fn(|i| {
            let state = &self.0[i];
            (
                state.started.load(SeqCst),
                state.finished.load(SeqCst),
                state.destroyed.load(SeqCst),
            )
        })
    }

    async fn exited(&self) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while self.counts() != [(1, 1, 1); 2] {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("both actual task futures must be destroyed");
    }
}

impl TaskGuard {
    pub(super) fn started(mut self) -> Self {
        self.started = true;
        self.probes.0[self.index]
            .started
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        // Passive observation only. Finished includes cancellation, not a claim
        // of graceful completion. Destruction also records abort before poll.
        let state = &self.probes.0[self.index];
        if self.started {
            state
                .finished
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        state
            .destroyed
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

struct PeerWrite(Vec<u8>, tokio::sync::oneshot::Sender<()>);

struct TurnFixture {
    turn: tokio::task::JoinHandle<Result<()>>,
    peer: tokio::task::JoinHandle<()>,
    consumer: tokio::task::JoinHandle<Vec<StreamEvent>>,
    writes: mpsc::Sender<PeerWrite>,
    requests: mpsc::UnboundedReceiver<Vec<u8>>,
    results: mpsc::Sender<jcode_provider_core::NativeToolResult>,
    probes: TaskProbes,
}

fn fixture_tools() -> Vec<jcode_message_types::ToolDefinition> {
    vec![jcode_message_types::ToolDefinition {
        name: "mcp__fixture-server__read.file".into(),
        description: "Synthetic registry fixture".into(),
        input_schema: serde_json::json!({"type": "object"}),
        defer_loading: false,
    }]
}

impl TurnFixture {
    async fn new(status: u16) -> Self {
        let (client, server) = tokio::io::duplex(65536);
        let (writes, mut commands) = mpsc::channel::<PeerWrite>(8);
        let (request_tx, requests) = mpsc::unbounded_channel();
        let peer = tokio::spawn(async move {
            let mut connection = h2::server::handshake(server).await.unwrap();
            let (request, mut respond) = connection.accept().await.unwrap().unwrap();
            assert_eq!(request.uri().path(), AGENT_PATH);
            let mut body = request.into_body();
            let mut response = respond
                .send_response(
                    http::Response::builder().status(status).body(()).unwrap(),
                    false,
                )
                .unwrap();
            let mut pending = Vec::new();
            let mut reading = true;
            let mut driving = true;
            loop {
                tokio::select! {
                    next = connection.accept(), if driving => {
                        assert!(!matches!(next, Some(Ok(_))), "one request per fixture");
                        driving = false;
                    }
                    chunk = body.data(), if reading => {
                        match chunk {
                            Some(Ok(chunk)) => {
                                body.flow_control().release_capacity(chunk.len()).unwrap();
                                pending.extend_from_slice(&chunk);
                                while let Some((_, payload, consumed)) = next_frame(&pending).unwrap() {
                                    pending.drain(..consumed);
                                    request_tx.send(payload).unwrap();
                                }
                            }
                            Some(Err(_)) | None => reading = false,
                        }
                    }
                    command = commands.recv() => {
                        let Some(PeerWrite(bytes, delivered)) = command else { break };
                        assert!(bytes.len() < 16384, "single bounded HTTP/2 DATA write");
                        response.send_data(Bytes::from(bytes), false).unwrap();
                        delivered.send(()).unwrap();
                    }
                }
            }
        });
        let (tx, mut events) = mpsc::channel::<Result<StreamEvent>>(128);
        let consumer = tokio::spawn(async move {
            let mut collected = Vec::new();
            while let Some(event) = events.recv().await {
                collected.push(event.unwrap());
            }
            collected
        });
        let (results, mut result_rx) = mpsc::channel(8);
        let probes = TaskProbes::default();
        let child_probes = probes.clone();
        let turn = tokio::spawn(async move {
            run_agent_turn_on_io(
                client,
                "fixture.invalid",
                "/",
                "synthetic",
                "hello",
                "composer-2.5",
                None,
                &[],
                None,
                "fixture-stream",
                &fixture_tools(),
                "",
                &mut result_rx,
                tx,
                child_probes,
            )
            .await
        });
        let mut fixture = Self {
            turn,
            peer,
            consumer,
            writes,
            requests,
            results,
            probes,
        };
        let first = fixture.request(Duration::from_secs(2)).await;
        assert!(iter_fields(&first).any(|f| f.field == 1));
        assert_eq!(fixture.probes.counts(), [(1, 0, 0); 2]);
        fixture
    }

    async fn send(&self, bytes: Vec<u8>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.writes.send(PeerWrite(bytes, tx)).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .unwrap()
            .unwrap();
    }

    async fn request(&mut self, budget: Duration) -> Vec<u8> {
        tokio::time::timeout(budget, self.requests.recv())
            .await
            .expect("peer must observe actual request DATA")
            .unwrap()
    }

    async fn finish(mut self) -> (Result<()>, Vec<StreamEvent>, Vec<Vec<u8>>) {
        let result = tokio::time::timeout(Duration::from_secs(2), &mut self.turn)
            .await
            .expect("turn must terminate within fixture budget")
            .unwrap();
        self.probes.exited().await;
        let events = (&mut self.consumer).await.unwrap();
        // Frames already buffered in h2 may be observed after task completion.
        // Only passive task destruction proves that new writes have stopped.
        tokio::time::sleep(Duration::from_millis(20)).await;
        let mut requests = Vec::new();
        while let Ok(payload) = self.requests.try_recv() {
            requests.push(payload);
        }
        self.peer.abort();
        assert!((&mut self.peer).await.unwrap_err().is_cancelled());
        (result, events, requests)
    }
}

impl Drop for TurnFixture {
    fn drop(&mut self) {
        self.turn.abort();
        self.peer.abort();
        self.consumer.abort();
    }
}

fn fixture_exec(field: Option<u64>, args: &[u8]) -> Vec<u8> {
    let mut exec = field_varint(1, 73);
    exec.extend(field_str(15, "PRIVATE_EXEC_MARKER"));
    if let Some(field) = field {
        exec.extend(field_ld(field, args));
    }
    connect_frame(&field_ld(2, &exec))
}

fn fixture_mcp() -> Vec<u8> {
    let tools = fixture_tools();
    let aliases = crate::wire::mcp_wire_aliases(&tools);
    let mut args = field_str(1, &aliases[&tools[0].name]);
    args.extend(field_str(4, "ccbridge"));
    args.extend(field_str(5, &tools[0].name));
    fixture_exec(Some(11), &args)
}

fn fixture_trailer(error: bool) -> Vec<u8> {
    let mut frame = connect_frame(if error {
        br#"{"error":{"code":"fixture"}}"#
    } else {
        b"{}"
    });
    frame[0] = 2;
    frame
}

async fn healthy_turn() {
    let fixture = TurnFixture::new(200).await;
    let mut bytes = connect_frame(&field_ld(1, &field_ld(1, &field_str(1, "answer"))));
    bytes.extend(fixture_trailer(false));
    fixture.send(bytes).await;
    let (result, events, _) = fixture.finish().await;
    result.expect("healthy HTTP/2 peer completes shared production path");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StreamEvent::TextDelta(s) if s == "answer"))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StreamEvent::MessageEnd { .. }))
    );
}

async fn malformed_cases(invalid_utf8: bool) {
    for prefix in [None, Some(1), Some(4)] {
        for held_open in [false, true] {
            let fixture = TurnFixture::new(200).await;
            let mut bytes = Vec::new();
            if let Some(branch) = prefix {
                bytes.extend(connect_frame(&field_ld(
                    1,
                    &field_ld(branch, &field_str(1, "prior")),
                )));
            }
            bytes.extend(if invalid_utf8 {
                fixture_exec(Some(42), &field_ld(1, b"PRIVATE_PROVIDER_MARKER\xff"))
            } else {
                fixture_exec(None, &[])
            });
            bytes.extend(fixture_mcp());
            if !held_open {
                bytes.extend(fixture_trailer(false));
            }
            fixture.send(bytes).await;
            let (result, events, requests) = fixture.finish().await;
            assert!(
                result.is_err(),
                "malformed execution must not become a successful turn"
            );
            let diagnostic = format!("{:#}", result.unwrap_err());
            assert_eq!(diagnostic, "Malformed Cursor execution request");
            assert!(!diagnostic.contains("PRIVATE_"));
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e, StreamEvent::NativeToolCall { .. }))
            );
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e, StreamEvent::MessageEnd { .. }))
            );
            assert!(
                !requests
                    .iter()
                    .any(|p| iter_fields(p).any(|f| f.field == 2))
            );
            if prefix == Some(4) {
                assert!(events.iter().any(|e| matches!(e, StreamEvent::ThinkingEnd)));
            }
        }
    }
    healthy_turn().await;
}

#[tokio::test]
async fn exec_decode_error_terminates_transport_without_dispatch_or_message_end() {
    malformed_cases(false).await;
}

#[tokio::test]
async fn exec_invalid_provider_utf8_terminates_transport_without_dispatch_or_message_end() {
    malformed_cases(true).await;
}

#[tokio::test]
async fn exec_transport_controls_preserve_defaults_unknown_and_registry_dispatch() {
    healthy_turn().await;
    for provider in [None, Some(""), Some("other"), Some("ccbridge")] {
        let mut fixture = TurnFixture::new(200).await;
        let args = provider.map(|p| field_str(1, p)).unwrap_or_default();
        fixture.send(fixture_exec(Some(42), &args)).await;
        let reply = fixture.request(Duration::from_secs(2)).await;
        let expected = crate::wire::encode_agent_client_exec_message(
            &crate::wire::encode_allowlist_precheck_result(
                73,
                "PRIVATE_EXEC_MARKER",
                42,
                provider == Some("ccbridge"),
            ),
        );
        assert_eq!(reply, expected);
        fixture.send(fixture_trailer(false)).await;
        fixture.finish().await.0.unwrap();
    }
    for field in [
        2, 3, 4, 5, 7, 8, 9, 10, 14, 16, 17, 18, 20, 21, 22, 23, 36, 41, 43,
    ] {
        let mut fixture = TurnFixture::new(200).await;
        let frame = fixture_exec(Some(field), &[]);
        let (_, payload, _) = next_frame(&frame).unwrap().unwrap();
        let exec = iter_fields(&payload).find(|f| f.field == 2).unwrap();
        let decoded = crate::wire::decode_exec_server_message(exec.data).unwrap();
        let expected = crate::wire::encode_control_exec_reply(&decoded, &fixture_tools()).unwrap();
        fixture.send(frame).await;
        assert_eq!(fixture.request(Duration::from_secs(2)).await, expected);
        let outer = iter_fields(&expected).collect::<Vec<_>>();
        assert_eq!(outer.len(), 1);
        assert_eq!(outer[0].field, 2);
        assert!(iter_fields(outer[0].data).any(|f| f.field == field));
        assert!(!iter_fields(outer[0].data).any(|f| f.field == 11));
        fixture.send(fixture_trailer(false)).await;
        fixture.finish().await.0.unwrap();
    }
    let mut fixture = TurnFixture::new(200).await;
    fixture.send(fixture_exec(Some(99), &[])).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(100), fixture.requests.recv())
            .await
            .is_err()
    );
    assert!(
        !fixture.turn.is_finished(),
        "unknown variants retain progress deadline"
    );
    fixture.send(fixture_mcp()).await;
    let request_id = "fixture-stream:73:PRIVATE_EXEC_MARKER".to_string();
    fixture
        .results
        .send(jcode_provider_core::NativeToolResult::success(
            request_id.clone(),
            "synthetic safe result".into(),
        ))
        .await
        .unwrap();
    let reply = fixture.request(Duration::from_secs(2)).await;
    assert_eq!(
        reply,
        crate::wire::encode_agent_client_exec_message(&crate::wire::encode_mcp_success_result(
            73,
            "PRIVATE_EXEC_MARKER",
            "synthetic safe result",
            false
        ),)
    );
    const SECRET: &str = "sk-ant-oat01-ABCDEFGHIJKLMNOPQRSTUVWX";
    let redacted = jcode_base::message::redact_secrets(SECRET);
    assert_ne!(
        redacted, SECRET,
        "fixture must exercise established redaction"
    );
    fixture.send(fixture_mcp()).await;
    fixture
        .results
        .send(jcode_provider_core::NativeToolResult::success(
            request_id.clone(),
            SECRET.into(),
        ))
        .await
        .unwrap();
    let private_reply = fixture.request(Duration::from_secs(2)).await;
    assert!(
        !private_reply
            .windows(SECRET.len())
            .any(|w| w == SECRET.as_bytes())
    );
    assert_eq!(
        private_reply,
        crate::wire::encode_agent_client_exec_message(&crate::wire::encode_mcp_success_result(
            73,
            "PRIVATE_EXEC_MARKER",
            &redacted,
            false
        ),)
    );
    fixture.send(fixture_trailer(false)).await;
    let (result, events, _) = fixture.finish().await;
    result.unwrap();
    assert!(events.iter().any(
        |e| matches!(e, StreamEvent::NativeToolCall { request_id: id, tool_name, .. }
        if id == &request_id && tool_name == &fixture_tools()[0].name)
    ));
    for (status, trailer_error) in [(503, false), (503, true), (200, true)] {
        let fixture = TurnFixture::new(status).await;
        fixture.send(fixture_trailer(trailer_error)).await;
        let error = fixture.finish().await.0.unwrap_err().to_string();
        assert!(error.contains(if trailer_error {
            "stream error"
        } else {
            "HTTP 503"
        }));
    }
}

#[tokio::test]
async fn exec_transport_owned_tasks_exit_on_malformed_and_cancellation() {
    // Poll the actual owner without yielding to its children on this current-thread
    // runtime. Dropping it must destroy captures even before either first poll.
    let (client, _server) = tokio::io::duplex(65536);
    let (tx, _events) = mpsc::channel(128);
    let (_results, mut result_rx) = mpsc::channel(8);
    let probes = TaskProbes::default();
    let tools = fixture_tools();
    let mut turn = Box::pin(run_agent_turn_on_io(
        client,
        "fixture.invalid",
        "/",
        "synthetic",
        "hello",
        "composer-2.5",
        None,
        &[],
        None,
        "fixture-stream",
        &tools,
        "",
        &mut result_rx,
        tx,
        probes.clone(),
    ));
    std::future::poll_fn(|cx| {
        assert!(std::future::Future::poll(turn.as_mut(), cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    assert_eq!(probes.counts(), [(0, 0, 0); 2]);
    drop(turn);
    tokio::time::timeout(Duration::from_secs(2), async {
        while probes.counts() != [(0, 0, 1); 2] {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("both unpolled actual child futures must be destroyed");

    for heartbeat in [false, true] {
        for cancel in [false, true] {
            let mut fixture = TurnFixture::new(200).await;
            if heartbeat {
                assert_eq!(
                    fixture.request(Duration::from_secs(8)).await,
                    field_ld(7, &[])
                );
                assert!(
                    !fixture.turn.is_finished(),
                    "heartbeat is not a terminal frame"
                );
            }
            if cancel {
                fixture.turn.abort();
                assert!((&mut fixture.turn).await.unwrap_err().is_cancelled());
                fixture.probes.exited().await;
            } else {
                let probes = fixture.probes.clone();
                fixture.send(fixture_exec(None, &[])).await;
                let (result, events, _) = fixture.finish().await;
                assert_eq!(
                    result.unwrap_err().to_string(),
                    "Malformed Cursor execution request"
                );
                assert!(
                    !events
                        .iter()
                        .any(|e| matches!(e, StreamEvent::MessageEnd { .. }))
                );
                probes.exited().await;
            }
        }
    }
    // Consumer loss is separate from the malformed-input discriminators.
    let mut fixture = TurnFixture::new(200).await;
    fixture.consumer.abort();
    assert!((&mut fixture.consumer).await.unwrap_err().is_cancelled());
    fixture
        .send(connect_frame(&field_ld(
            1,
            &field_ld(1, &field_str(1, "answer")),
        )))
        .await;
    let result = tokio::time::timeout(Duration::from_secs(2), &mut fixture.turn)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    fixture.probes.exited().await;
}
