use super::super::*;
use crate::message::{Message, StreamEvent, ToolDefinition};
use crate::provider::{EventStream, Provider};
use crate::tool::{Registry, Tool, ToolContext, ToolOutput};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

#[derive(Clone)]
struct ToolSpec {
    id: &'static str,
    name: &'static str,
    input: Value,
    signature: Option<&'static str>,
    raw_input: Option<&'static str>,
    sdk_result: Option<(&'static str, bool)>,
}

#[derive(Clone)]
struct ScriptedProvider {
    state: Arc<ProviderState>,
}

struct ProviderState {
    calls: AtomicUsize,
    specs: Vec<ToolSpec>,
    continuation_messages: StdMutex<Option<Vec<Message>>>,
}

impl ScriptedProvider {
    fn new(specs: Vec<ToolSpec>) -> Self {
        Self {
            state: Arc::new(ProviderState {
                calls: AtomicUsize::new(0),
                specs,
                continuation_messages: StdMutex::new(None),
            }),
        }
    }

    fn continuation_messages(&self) -> Vec<Message> {
        self.state
            .continuation_messages
            .lock()
            .unwrap()
            .clone()
            .expect("provider continuation was not requested")
    }
}

#[async_trait]
impl Provider for ScriptedProvider {
    async fn complete(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let call = self.state.calls.fetch_add(1, Ordering::SeqCst);
        let mut events = Vec::new();
        if call == 0 {
            for spec in &self.state.specs {
                events.push(Ok(StreamEvent::ToolUseStart {
                    id: spec.id.to_string(),
                    name: spec.name.to_string(),
                }));
                events.push(Ok(StreamEvent::ToolInputDelta(
                    spec.raw_input
                        .map(ToString::to_string)
                        .unwrap_or_else(|| spec.input.to_string()),
                )));
                events.push(Ok(StreamEvent::ToolUseEnd));
                if let Some(signature) = spec.signature {
                    events.push(Ok(StreamEvent::ToolUseSignature(signature.to_string())));
                }
                if let Some((content, is_error)) = spec.sdk_result {
                    events.push(Ok(StreamEvent::ToolResult {
                        tool_use_id: spec.id.to_string(),
                        content: content.to_string(),
                        is_error,
                    }));
                }
            }
            events.push(Ok(StreamEvent::MessageEnd {
                stop_reason: Some("tool_use".to_string()),
            }));
        } else {
            *self.state.continuation_messages.lock().unwrap() = Some(messages.to_vec());
            events.push(Ok(StreamEvent::TextDelta("done".to_string())));
            events.push(Ok(StreamEvent::MessageEnd {
                stop_reason: Some("end_turn".to_string()),
            }));
        }
        Ok(Box::pin(tokio_stream::iter(events)))
    }

    fn name(&self) -> &str {
        "scripted-concurrency"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

#[derive(Default)]
struct DelayedToolState {
    active: AtomicUsize,
    max_active: AtomicUsize,
    starts: StdMutex<Vec<String>>,
    completions: StdMutex<Vec<String>>,
    aborted: StdMutex<Vec<String>>,
    gates: StdMutex<HashMap<String, Arc<Notify>>>,
    changed: Notify,
}

impl DelayedToolState {
    fn gate(&self, id: &str) -> Arc<Notify> {
        self.gates
            .lock()
            .unwrap()
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }

    fn release(&self, id: &str) {
        self.gate(id).notify_one();
    }

    async fn wait_for_starts(&self, count: usize) {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if self.starts.lock().unwrap().len() >= count {
                    break;
                }
                self.changed.notified().await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "timed out waiting for {count} starts; got {:?}",
                self.starts.lock().unwrap()
            )
        });
    }

    async fn wait_for_completions(&self, count: usize) {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if self.completions.lock().unwrap().len() >= count {
                    break;
                }
                self.changed.notified().await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "timed out waiting for {count} completions; got {:?}",
                self.completions.lock().unwrap()
            )
        });
    }

    fn started(&self, id: &str) -> bool {
        self.starts.lock().unwrap().iter().any(|item| item == id)
    }
}

struct ActiveGuard {
    state: Arc<DelayedToolState>,
    id: String,
    completed: bool,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.state.active.fetch_sub(1, Ordering::SeqCst);
        if !self.completed {
            self.state.aborted.lock().unwrap().push(self.id.clone());
        }
        self.state.changed.notify_one();
    }
}

struct DelayedTool {
    name: &'static str,
    state: Arc<DelayedToolState>,
}

#[async_trait]
impl Tool for DelayedTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "Deterministic delayed tool for native turn concurrency tests."
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let id = input["id"].as_str().unwrap().to_string();
        let active = self.state.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.state.max_active.fetch_max(active, Ordering::SeqCst);
        self.state.starts.lock().unwrap().push(id.clone());
        self.state.changed.notify_one();
        let mut guard = ActiveGuard {
            state: self.state.clone(),
            id: id.clone(),
            completed: false,
        };

        match input["behavior"].as_str().unwrap_or("wait") {
            "error" => return Err(anyhow!("requested error {id}")),
            "panic" => panic!("requested panic {id}"),
            "wait_error" => {
                self.state.gate(&id).notified().await;
                return Err(anyhow!("requested error {id}"));
            }
            "wait_panic" => {
                self.state.gate(&id).notified().await;
                panic!("requested panic {id}");
            }
            "ready" => {}
            _ => self.state.gate(&id).notified().await,
        }

        guard.completed = true;
        self.state.completions.lock().unwrap().push(id.clone());
        Ok(ToolOutput::new(format!("output-{id}")))
    }
}

/// Suppresses the host's configured hooks for the duration of a test.
///
/// `tool_concurrency::plan` treats any configured `pre_tool`/`post_tool` hook as
/// a serial barrier, so a developer's live `~/.jcode/config.toml` `[hooks]` block
/// would otherwise serialize every overlap test and time out `wait_for_starts`.
/// Callers hold `lock_test_env` already; this only flips the recursion-guard
/// variable the hook runner honours and restores it on drop.
struct HooksOff(Option<std::ffi::OsString>);

impl HooksOff {
    fn new() -> Self {
        let previous = std::env::var_os("JCODE_HOOKS_DISABLED");
        crate::env::set_var("JCODE_HOOKS_DISABLED", "1");
        crate::config::invalidate_config_cache();
        Self(previous)
    }
}

impl Drop for HooksOff {
    fn drop(&mut self) {
        match self.0.take() {
            Some(value) => crate::env::set_var("JCODE_HOOKS_DISABLED", value),
            None => crate::env::remove_var("JCODE_HOOKS_DISABLED"),
        }
        crate::config::invalidate_config_cache();
    }
}

async fn agent_with_tools(
    specs: Vec<ToolSpec>,
) -> (Agent, ScriptedProvider, Arc<DelayedToolState>) {
    let provider = ScriptedProvider::new(specs);
    let provider_dyn: Arc<dyn Provider> = Arc::new(provider.clone());
    let registry = Registry::empty();
    let state = Arc::new(DelayedToolState::default());
    for name in ["read", "ls", "jcode_docs", "write", "bash"] {
        registry
            .register(
                name.to_string(),
                Arc::new(DelayedTool {
                    name,
                    state: state.clone(),
                }),
            )
            .await;
    }
    (Agent::new(provider_dyn, registry), provider, state)
}

fn spec(id: &'static str, name: &'static str, file_path: &'static str) -> ToolSpec {
    ToolSpec {
        id,
        name,
        input: json!({"id": id, "file_path": file_path, "intent": format!("run {id}")}),
        signature: Some(match id {
            "one" => "sig-one",
            "two" => "sig-two",
            "three" => "sig-three",
            "four" => "sig-four",
            _ => "sig-five",
        }),
        raw_input: None,
        sdk_result: None,
    }
}

fn tool_results_from_blocks<'a>(
    blocks: impl Iterator<Item = &'a ContentBlock>,
) -> Vec<(String, String, bool)> {
    blocks
        .filter_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => Some((
                tool_use_id.clone(),
                content.clone(),
                is_error == &Some(true),
            )),
            _ => None,
        })
        .collect()
}

fn tool_results(messages: &[Message]) -> Vec<(String, String, bool)> {
    tool_results_from_blocks(messages.iter().flat_map(|message| &message.content))
}

fn stored_tool_results(messages: &[StoredMessage]) -> Vec<(String, String, bool)> {
    tool_results_from_blocks(messages.iter().flat_map(|message| &message.content))
}

fn stored_tool_durations(messages: &[StoredMessage]) -> HashMap<String, u64> {
    messages
        .iter()
        .flat_map(|message| {
            message.content.iter().filter_map(move |block| match block {
                ContentBlock::ToolResult { tool_use_id, .. } => message
                    .tool_duration_ms
                    .map(|duration| (tool_use_id.clone(), duration)),
                _ => None,
            })
        })
        .collect()
}

fn tool_signatures_from_blocks<'a>(
    blocks: impl Iterator<Item = &'a ContentBlock>,
) -> Vec<(String, Option<String>)> {
    blocks
        .filter_map(|block| match block {
            ContentBlock::ToolUse {
                id,
                thought_signature,
                ..
            } => Some((id.clone(), thought_signature.clone())),
            _ => None,
        })
        .collect()
}

fn stored_tool_signatures(messages: &[StoredMessage]) -> Vec<(String, Option<String>)> {
    tool_signatures_from_blocks(messages.iter().flat_map(|message| &message.content))
}

fn event_ids(events: &[ServerEvent], kind: &str) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match (kind, event) {
            ("start", ServerEvent::ToolStart { id, .. })
            | ("exec", ServerEvent::ToolExec { id, .. })
            | ("done", ServerEvent::ToolDone { id, .. }) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

fn background_task_id(content: &str) -> String {
    content
        .split_once("(task_id: ")
        .and_then(|(_, suffix)| suffix.split_once(')'))
        .map(|(task_id, _)| task_id.to_string())
        .unwrap_or_else(|| panic!("missing background task id in: {content}"))
}

async fn collect_events(mut rx: mpsc::UnboundedReceiver<ServerEvent>) -> Vec<ServerEvent> {
    tokio::time::timeout(Duration::from_secs(3), async move {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }
        events
    })
    .await
    .expect("timed out collecting server events")
}

async fn join_turn<T>(turn: tokio::task::JoinHandle<T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), turn)
        .await
        .expect("timed out waiting for agent turn")
        .expect("agent turn task panicked")
}

#[tokio::test]
async fn native_tool_concurrency_overlaps_caps_orders_aliases_and_signatures() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let specs = vec![
        spec("one", "Read", "one.RS"),
        spec("two", "functions.Read", "two.rs"),
        spec("three", "file_read", "three.rs"),
        spec("four", "read_file", "four.rs"),
        spec("five", "read", "five.rs"),
    ];
    let (mut agent, provider, state) = agent_with_tools(specs).await;
    let (tx, rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("run tools", Vec::new(), None, tx)
            .await;
        (agent, result)
    });

    state.wait_for_starts(4).await;
    assert_eq!(state.max_active.load(Ordering::SeqCst), 4);
    assert!(
        !state.started("five"),
        "cap must leave the fifth call queued"
    );
    for (completed, id) in ["four", "three", "two", "one"].into_iter().enumerate() {
        state.release(id);
        state.wait_for_completions(completed + 1).await;
        if id == "four" {
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
    }
    state.wait_for_starts(5).await;
    state.release("five");
    state.wait_for_completions(5).await;

    let (agent, result) = join_turn(turn).await;
    result.unwrap();
    let events = collect_events(rx).await;
    let expected = vec!["one", "two", "three", "four", "five"];
    assert_eq!(event_ids(&events, "start"), expected);
    assert_eq!(event_ids(&events, "exec"), expected);
    assert_eq!(event_ids(&events, "done"), expected);
    assert_eq!(
        state.completions.lock().unwrap().as_slice(),
        ["four", "three", "two", "one", "five"]
    );
    let durations = stored_tool_durations(&agent.session.messages);
    assert!(
        durations["one"] >= 50,
        "first call should include its gated wait"
    );
    assert!(
        durations["four"] < 50,
        "completed call duration must not include waiting to persist provider order: {durations:?}"
    );
    let continuation = provider.continuation_messages();
    assert_eq!(
        tool_results(&continuation)
            .into_iter()
            .map(|(id, _, _)| id)
            .collect::<Vec<_>>(),
        expected
    );
    drop(agent);
}

#[tokio::test]
async fn native_tool_concurrency_preserves_streaming_thought_signature() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let mut ready = spec("one", "read", "one.rs");
    ready.input["behavior"] = json!("ready");
    let (mut agent, _provider, _state) = agent_with_tools(vec![ready]).await;
    let (tx, _rx) = mpsc::unbounded_channel();

    tokio::time::timeout(
        Duration::from_secs(3),
        agent.run_once_streaming_mpsc("preserve signature", Vec::new(), None, tx),
    )
    .await
    .expect("timed out waiting for signature turn")
    .unwrap();

    assert_eq!(
        stored_tool_signatures(&agent.session.messages),
        vec![("one".into(), Some("sig-one".into()))]
    );
}

#[tokio::test]
async fn native_tool_concurrency_overlaps_on_blocking_turn_path() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let specs = vec![
        spec("one", "read", "one.rs"),
        spec("two", "ls", "."),
        spec("three", "jcode_docs", "docs"),
        spec("four", "read", "four.rs"),
        spec("five", "read", "five.rs"),
    ];
    let (mut agent, provider, state) = agent_with_tools(specs).await;
    let turn = tokio::spawn(async move {
        let result = agent.run_once("run blocking tools").await;
        (agent, result)
    });

    state.wait_for_starts(4).await;
    assert_eq!(state.max_active.load(Ordering::SeqCst), 4);
    assert!(
        !state.started("five"),
        "cap must leave the fifth call queued"
    );
    for (completed, id) in ["four", "three", "two", "one"].into_iter().enumerate() {
        state.release(id);
        state.wait_for_completions(completed + 1).await;
    }
    state.wait_for_starts(5).await;
    state.release("five");
    state.wait_for_completions(5).await;

    let (agent, result) = join_turn(turn).await;
    result.unwrap();
    assert_eq!(
        state.completions.lock().unwrap().as_slice(),
        ["four", "three", "two", "one", "five"]
    );
    let expected = vec!["one", "two", "three", "four", "five"];
    assert_eq!(
        tool_results(&provider.continuation_messages())
            .into_iter()
            .map(|(id, _, _)| id)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        stored_tool_results(&agent.session.messages)
            .into_iter()
            .map(|(id, _, _)| id)
            .collect::<Vec<_>>(),
        expected
    );
}

#[tokio::test]
async fn native_tool_concurrency_respects_mutation_and_extension_barriers() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let specs = vec![
        spec("one", "read", "one.rs"),
        spec("two", "file_read", "two.rs"),
        spec("three", "Read", "image.PnG"),
        spec("four", "functions.Read", "report.PDF"),
        spec("five", "write", "out.rs"),
        spec("six", "ls", "."),
        spec("seven", "jcode_docs", "docs"),
    ];
    let (mut agent, _provider, state) = agent_with_tools(specs).await;
    let (tx, _rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("run barriers", Vec::new(), None, tx)
            .await;
        (agent, result)
    });

    state.wait_for_starts(2).await;
    assert!(!state.started("three"));
    state.release("two");
    state.release("one");
    state.wait_for_starts(3).await;
    assert!(!state.started("four"));
    state.release("three");
    state.wait_for_starts(4).await;
    assert!(!state.started("five"));
    state.release("four");
    state.wait_for_starts(5).await;
    assert!(!state.started("six"));
    state.release("five");
    state.wait_for_starts(7).await;
    state.release("seven");
    state.release("six");

    let (_, result) = join_turn(turn).await;
    result.unwrap();
}

#[tokio::test]
async fn native_tool_concurrency_treats_sdk_validation_and_unknown_as_barriers() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let one = spec("one", "read", "one.rs");
    let mut two = spec("two", "read", "two.rs");
    two.sdk_result = Some(("sdk-two", false));
    let mut three = spec("three", "read", "three.rs");
    three.raw_input = Some("{");
    let four = spec("four", "unknown_tool", ".");
    let five = spec("five", "read", "five.rs");
    let (mut agent, provider, state) = agent_with_tools(vec![one, two, three, four, five]).await;
    let (tx, _rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("serial barriers", Vec::new(), None, tx)
            .await;
        (agent, result)
    });

    state.wait_for_starts(1).await;
    assert!(!state.started("five"));
    state.release("one");
    state.wait_for_completions(1).await;
    state.wait_for_starts(2).await;
    assert_eq!(state.starts.lock().unwrap().as_slice(), ["one", "five"]);
    state.release("five");

    let (_agent, result) = join_turn(turn).await;
    result.unwrap();
    let results = tool_results(&provider.continuation_messages());
    assert_eq!(
        results
            .iter()
            .map(|(id, _, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["one", "two", "three", "four", "five"]
    );
    assert!(
        results
            .iter()
            .any(|(id, text, error)| { id == "two" && text.contains("sdk-two") && !*error }),
        "unexpected SDK barrier results: {results:?}"
    );
    assert!(results.iter().any(|(id, text, error)| {
        id == "three" && *error && text.contains("arguments must be a JSON object")
    }));
    assert!(
        results
            .iter()
            .any(|(id, text, error)| { id == "four" && *error && text.contains("Unknown tool") })
    );
}

#[cfg(unix)]
#[tokio::test]
async fn native_tool_concurrency_treats_configured_hooks_as_serial_barriers() {
    use std::os::unix::fs::PermissionsExt;

    struct HookEnvGuard(Option<std::ffi::OsString>);
    impl Drop for HookEnvGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => crate::env::set_var("JCODE_HOOK_PRE_TOOL", value),
                None => crate::env::remove_var("JCODE_HOOK_PRE_TOOL"),
            }
            crate::config::invalidate_config_cache();
        }
    }

    let _guard = crate::storage::lock_test_env();
    let temp = tempfile::TempDir::new().expect("temp dir");
    let hook = temp.path().join("allow.sh");
    std::fs::write(&hook, "#!/bin/sh\ncat >/dev/null\nexit 0\n").expect("write hook");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod hook");
    let hook_guard = HookEnvGuard(std::env::var_os("JCODE_HOOK_PRE_TOOL"));
    crate::env::set_var("JCODE_HOOK_PRE_TOOL", hook.to_string_lossy().to_string());
    crate::config::invalidate_config_cache();

    let (mut agent, _provider, state) =
        agent_with_tools(vec![spec("one", "read", "one.rs"), spec("two", "ls", ".")]).await;
    let (tx, _rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("hook barrier", Vec::new(), None, tx)
            .await;
        (agent, result)
    });

    state.wait_for_starts(1).await;
    assert!(!state.started("two"));
    state.release("one");
    state.wait_for_starts(2).await;
    state.release("two");
    let (_, result) = join_turn(turn).await;
    result.unwrap();
    drop(hook_guard);
}

#[tokio::test]
async fn native_tool_concurrency_terminalizes_errors_and_panics_once() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let mut one = spec("one", "read", "one.rs");
    one.input["behavior"] = json!("ready");
    let mut two = spec("two", "ls", ".");
    two.input["behavior"] = json!("error");
    let mut three = spec("three", "jcode_docs", "docs");
    three.input["behavior"] = json!("panic");
    let (mut agent, provider, _state) = agent_with_tools(vec![one, two, three]).await;
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::time::timeout(
        Duration::from_secs(3),
        agent.run_once_streaming_mpsc("run failures", Vec::new(), None, tx),
    )
    .await
    .expect("timed out waiting for failure turn")
    .unwrap();
    let events = collect_events(rx).await;
    assert_eq!(event_ids(&events, "done"), ["one", "two", "three"]);
    let results = tool_results(&provider.continuation_messages());
    assert_eq!(results.len(), 3);
    assert!(!results[0].2);
    assert!(results[1].2 && results[1].1.contains("requested error"));
    assert!(results[2].2 && results[2].1.contains("Tool task panicked"));
}

#[tokio::test]
async fn native_tool_concurrency_parent_abort_drops_all_child_reads() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let specs = vec![
        spec("one", "read", "one.rs"),
        spec("two", "ls", "."),
        spec("three", "jcode_docs", "docs"),
        spec("four", "read", "four.rs"),
        spec("five", "read", "five.rs"),
        spec("six", "write", "out.rs"),
    ];
    let (mut agent, _provider, state) = agent_with_tools(specs).await;
    let session_id = agent.session.id.clone();
    let (tx, _rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        agent
            .run_once_streaming_mpsc("abort parent", Vec::new(), None, tx)
            .await
    });
    state.wait_for_starts(4).await;
    assert!(!state.started("five"));
    assert!(!state.started("six"));
    turn.abort();
    let join_error = tokio::time::timeout(Duration::from_secs(3), turn)
        .await
        .expect("timed out waiting for aborted parent")
        .expect_err("aborted parent unexpectedly completed");
    assert!(join_error.is_cancelled());
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.active.load(Ordering::SeqCst) != 0 {
            state.changed.notified().await;
        }
    })
    .await
    .expect("child tools remained active after parent abort");
    assert!(state.completions.lock().unwrap().is_empty());
    assert_eq!(state.aborted.lock().unwrap().len(), 4);
    for id in ["one", "two", "three", "four", "five", "six"] {
        assert!(!crate::tool::inflight::is_tool_in_flight(id));
    }
    drop(session_id);
}

#[tokio::test]
async fn native_tool_concurrency_reload_fills_every_result_once() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let specs = vec![
        spec("one", "read", "one.rs"),
        spec("two", "ls", "."),
        spec("three", "jcode_docs", "docs"),
        spec("four", "write", "out.rs"),
    ];
    let (mut agent, _provider, state) = agent_with_tools(specs).await;
    let shutdown = agent.graceful_shutdown_signal();
    let (tx, rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("reload", Vec::new(), None, tx)
            .await;
        (agent, result)
    });
    state.wait_for_starts(3).await;
    shutdown.fire();
    let (mut agent, result) = join_turn(turn).await;
    result.unwrap();
    let events = collect_events(rx).await;
    let results = stored_tool_results(&agent.session.messages);
    assert_eq!(results.len(), 4);
    for id in ["one", "two", "three", "four"] {
        assert_eq!(results.iter().filter(|(got, _, _)| got == id).count(), 1);
        assert_eq!(
            event_ids(&events, "done")
                .iter()
                .filter(|got| *got == id)
                .count(),
            1
        );
    }
    assert_eq!(agent.repair_missing_tool_outputs(), 0);
}

#[tokio::test]
async fn native_tool_concurrency_blocking_reload_preserves_completed_outcomes() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let one = spec("one", "read", "one.rs");
    let mut two = spec("two", "ls", ".");
    two.input["behavior"] = json!("ready");
    let three = spec("three", "write", "out.rs");
    let (mut agent, _provider, state) = agent_with_tools(vec![one, two, three]).await;
    let shutdown = agent.graceful_shutdown_signal();
    let turn = tokio::spawn(async move {
        let result = agent.run_once("blocking reload").await;
        (agent, result)
    });

    state.wait_for_starts(2).await;
    state.wait_for_completions(1).await;
    assert_eq!(state.completions.lock().unwrap().as_slice(), ["two"]);
    shutdown.fire();

    let (agent, result) = join_turn(turn).await;
    result.unwrap();
    let results = stored_tool_results(&agent.session.messages);
    assert_eq!(results.iter().filter(|(id, _, _)| id == "one").count(), 1);
    assert!(
        results.iter().any(|(id, text, error)| {
            id == "one" && *error && text.contains("server reloading")
        })
    );
    assert_eq!(
        results
            .iter()
            .find(|(id, _, _)| id == "two")
            .map(|(_, text, error)| (text.as_str(), *error)),
        Some(("output-two", false))
    );
    assert_eq!(results.iter().filter(|(id, _, _)| id == "three").count(), 1);
    assert!(!state.started("three"));
}

#[tokio::test]
async fn native_tool_concurrency_preserves_serial_bash_reload_handoff() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let (mut agent, _provider, state) = agent_with_tools(vec![spec("one", "bash", ".")]).await;
    let shutdown = agent.graceful_shutdown_signal();
    let (tx, _rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("bash reload handoff", Vec::new(), None, tx)
            .await;
        (agent, result)
    });

    state.wait_for_starts(1).await;
    shutdown.fire();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        state.active.load(Ordering::SeqCst),
        1,
        "bash must remain owned during the post-shutdown handoff window"
    );
    state.release("one");
    state.wait_for_completions(1).await;

    let (agent, result) = join_turn(turn).await;
    result.unwrap();
    assert!(
        stored_tool_results(&agent.session.messages)
            .iter()
            .any(|(id, text, error)| id == "one" && text == "output-one" && !*error)
    );
}

#[tokio::test]
async fn native_tool_concurrency_alt_b_backgrounds_each_unfinished_call_once() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let specs = vec![
        spec("one", "read", "one.rs"),
        spec("two", "ls", "."),
        spec("three", "jcode_docs", "docs"),
        spec("four", "read", "four.rs"),
        spec("five", "read", "five.rs"),
    ];
    let (mut agent, provider, state) = agent_with_tools(specs).await;
    let background = agent.background_tool_signal();
    let (tx, rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("background", Vec::new(), None, tx)
            .await;
        (agent, result)
    });
    state.wait_for_starts(4).await;
    assert!(!state.started("five"));
    background.fire();
    let (mut agent, result) = join_turn(turn).await;
    result.unwrap();
    let events = collect_events(rx).await;
    let results = tool_results(&provider.continuation_messages());
    assert_eq!(results.len(), 5);
    let mut task_ids = HashMap::new();
    for id in ["one", "two", "three", "four", "five"] {
        let matching = results
            .iter()
            .filter(|(got, _, _)| got == id)
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1);
        task_ids.insert(id, background_task_id(&matching[0].1));
        assert_eq!(
            event_ids(&events, "done")
                .iter()
                .filter(|got| *got == id)
                .count(),
            1
        );
    }
    assert_eq!(task_ids.values().collect::<HashSet<_>>().len(), 5);
    for id in ["five", "one", "two", "three", "four"] {
        let task_id = &task_ids[id];
        assert!(
            tokio::time::timeout(
                Duration::from_secs(3),
                crate::background::global().cancel(task_id),
            )
            .await
            .expect("timed out cancelling adopted task")
            .expect("background cancellation failed")
        );
        let status = crate::background::global()
            .status(task_id)
            .await
            .expect("cancelled task status");
        assert_eq!(status.error.as_deref(), Some("Cancelled by user"));
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.active.load(Ordering::SeqCst) != 0 {
            state.changed.notified().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(state.aborted.lock().unwrap().len(), 4);
    assert!(!state.started("five"));
    for id in ["one", "two", "three", "four", "five"] {
        assert!(!crate::tool::inflight::is_tool_in_flight(id));
    }
    assert_eq!(agent.repair_missing_tool_outputs(), 0);
}

#[tokio::test]
async fn native_tool_concurrency_background_failures_keep_status_and_output() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let mut one = spec("one", "read", "one.rs");
    one.input["behavior"] = json!("wait_error");
    let mut two = spec("two", "ls", ".");
    two.input["behavior"] = json!("wait_panic");
    let (mut agent, provider, state) = agent_with_tools(vec![one, two]).await;
    let background = agent.background_tool_signal();
    let (tx, _rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("background failures", Vec::new(), None, tx)
            .await;
        (agent, result)
    });

    state.wait_for_starts(2).await;
    background.fire();
    let (_agent, result) = join_turn(turn).await;
    result.unwrap();
    let task_ids = tool_results(&provider.continuation_messages())
        .into_iter()
        .map(|(id, content, _)| (id, background_task_id(&content)))
        .collect::<HashMap<_, _>>();
    assert_eq!(task_ids.values().collect::<HashSet<_>>().len(), 2);

    state.release("one");
    state.release("two");
    for (id, expected) in [
        ("one", "requested error one"),
        ("two", "Tool task panicked"),
    ] {
        let task_id = &task_ids[id];
        let waited = crate::background::global()
            .wait(task_id, Duration::from_secs(3), false)
            .await
            .expect("background task disappeared");
        assert!(
            waited
                .task
                .error
                .as_deref()
                .is_some_and(|error| error.contains(expected))
        );
        let output = crate::background::global()
            .output(task_id)
            .await
            .unwrap_or_else(|| panic!("missing failure output for task {task_id}"));
        assert!(
            output.contains(expected),
            "unexpected failure output: {output}"
        );
    }
}

#[tokio::test]
async fn native_tool_concurrency_urgent_interrupt_skips_after_active_group() {
    let _guard = crate::storage::lock_test_env();
    let _hooks = HooksOff::new();
    let specs = vec![
        spec("one", "read", "one.rs"),
        spec("two", "ls", "."),
        spec("three", "write", "out.rs"),
    ];
    let (mut agent, _provider, state) = agent_with_tools(specs).await;
    let queue = agent.soft_interrupt_queue();
    let (tx, _rx) = mpsc::unbounded_channel();
    let turn = tokio::spawn(async move {
        let result = agent
            .run_once_streaming_mpsc("urgent", Vec::new(), None, tx)
            .await;
        (agent, result)
    });
    state.wait_for_starts(2).await;
    queue.lock().unwrap().push(SoftInterruptMessage {
        content: "stop remaining tools".to_string(),
        images: Vec::new(),
        urgent: true,
        source: SoftInterruptSource::User,
    });
    state.release("two");
    state.release("one");
    let (agent, result) = join_turn(turn).await;
    result.unwrap();
    assert!(!state.started("three"));
    let results = stored_tool_results(&agent.session.messages);
    assert_eq!(results.iter().filter(|(id, _, _)| id == "three").count(), 1);
    assert!(results.iter().any(|(id, text, error)| {
        id == "three" && *error && text.contains("Skipped: user interrupted")
    }));
}
