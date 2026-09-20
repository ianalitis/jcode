#![allow(clippy::await_holding_lock)]

use super::super::*;
use crate::agent::Agent;
use crate::message::{ContentBlock, Message, Role, StreamEvent, ToolDefinition};
use crate::provider::{EventStream, Provider};
use crate::server::{SessionAgents, SwarmMember, VersionedPlan};
use crate::tool::Registry;
use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};

const COORDINATOR_ID: &str = "e1-context-coordinator";
const SWARM_ID: &str = "e1-context-swarm";
const CAPTAIN_CANARY: &str = "CAPTAIN_TRANSCRIPT_CANARY_MUST_NOT_LEAK";
const GLOBAL_POLICY: &str = "E1_GLOBAL_REQUIRED_SAFETY_POLICY";
const ROOT_POLICY: &str = "E1_ROOT_REQUIRED_PROJECT_POLICY";
const NESTED_POLICY: &str = "E1_NESTED_REQUIRED_TASK_POLICY";
const NESTED_POLICY_REVISION: &str = "E1_REVISED_NESTED_REQUIRED_TASK_POLICY";

#[derive(Debug, Clone)]
struct CapturedRequest {
    messages: Vec<Message>,
    tools: Vec<ToolDefinition>,
    system_static: String,
    system_dynamic: String,
}

#[derive(Debug, Clone, Serialize)]
struct RequestReceipt {
    label: String,
    repeat: usize,
    mode: String,
    system_static_bytes: usize,
    system_dynamic_bytes: usize,
    tools_json_bytes: usize,
    messages_json_bytes: usize,
    captured_serialized_bytes: usize,
    system_static_hash: u64,
    system_dynamic_hash: u64,
    tools_hash: u64,
    messages_hash: u64,
    stable_prefix_hash: u64,
    tool_names: Vec<String>,
}

impl CapturedRequest {
    fn receipt(&self, label: &str, repeat: usize, mode: &str) -> RequestReceipt {
        let system_static_bytes = self.system_static.len();
        let system_dynamic_bytes = self.system_dynamic.len();
        let tools_json_bytes = serde_json::to_vec(&self.tools)
            .expect("serialize captured tools")
            .len();
        let messages_json_bytes = serde_json::to_vec(&self.messages)
            .expect("serialize captured messages")
            .len();
        let system_static_hash = jcode_provider_core::stable_hash_str(&self.system_static);
        let system_dynamic_hash = jcode_provider_core::stable_hash_str(&self.system_dynamic);
        let tools_hash = jcode_provider_core::stable_hash_json(&self.tools);
        RequestReceipt {
            label: label.to_string(),
            repeat,
            mode: mode.to_string(),
            system_static_bytes,
            system_dynamic_bytes,
            tools_json_bytes,
            messages_json_bytes,
            // This is the deterministic serialized size visible at the Provider
            // boundary. Provider adapters may add envelopes or transform roles,
            // so it is not a claim about actual wire bytes.
            captured_serialized_bytes: system_static_bytes
                + system_dynamic_bytes
                + tools_json_bytes
                + messages_json_bytes,
            system_static_hash,
            system_dynamic_hash,
            tools_hash,
            messages_hash: jcode_provider_core::stable_hash_json(&self.messages),
            stable_prefix_hash: jcode_provider_core::stable_hash_json(&(
                system_static_hash,
                tools_hash,
            )),
            tool_names: self.tools.iter().map(|tool| tool.name.clone()).collect(),
        }
    }

    fn rendered_messages(&self) -> String {
        serde_json::to_string(&self.messages).expect("render captured messages")
    }
}

#[derive(Clone)]
struct CaptureProvider {
    tx: mpsc::UnboundedSender<CapturedRequest>,
    model: Arc<StdMutex<String>>,
    effort: Arc<StdMutex<Option<String>>>,
}

impl CaptureProvider {
    fn new(tx: mpsc::UnboundedSender<CapturedRequest>) -> Self {
        Self {
            tx,
            model: Arc::new(StdMutex::new("gpt-5.6-sol".to_string())),
            effort: Arc::new(StdMutex::new(Some("high".to_string()))),
        }
    }
}

#[async_trait]
impl Provider for CaptureProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        self.complete_split(messages, tools, system, "", None).await
    }

    async fn complete_split(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        system_static: &str,
        system_dynamic: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        self.tx
            .send(CapturedRequest {
                messages: messages.to_vec(),
                tools: tools.to_vec(),
                system_static: system_static.to_string(),
                system_dynamic: system_dynamic.to_string(),
            })
            .expect("capture receiver stays alive");
        Ok(Box::pin(futures::stream::iter([
            Ok(StreamEvent::TokenUsage {
                input_tokens: Some(321),
                output_tokens: Some(12),
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            }),
            Ok(StreamEvent::TextDelta("done".to_string())),
            Ok(StreamEvent::MessageEnd { stop_reason: None }),
        ])))
    }

    fn name(&self) -> &str {
        "e1-capture"
    }

    fn model(&self) -> String {
        self.model.lock().expect("model lock").clone()
    }

    fn set_model(&self, model: &str) -> Result<()> {
        let model = model
            .split_once(':')
            .map(|(_, model)| model)
            .unwrap_or(model)
            .trim();
        *self.model.lock().expect("model lock") = model.to_string();
        Ok(())
    }

    fn reasoning_effort(&self) -> Option<String> {
        self.effort.lock().expect("effort lock").clone()
    }

    fn set_reasoning_effort(&self, effort: &str) -> Result<()> {
        *self.effort.lock().expect("effort lock") = Some(effort.to_string());
        Ok(())
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

struct EnvGuard {
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvGuard {
    fn isolated(home: &Path, runtime: &Path) -> Self {
        let mut guard = Self { saved: Vec::new() };
        guard.set_path("JCODE_HOME", home);
        guard.set_path("HOME", home);
        guard.set_path("XDG_CONFIG_HOME", &home.join("config"));
        guard.set_path("JCODE_RUNTIME_DIR", runtime);
        guard.set("JCODE_MEMORY_ENABLED", "false");
        guard.set("JCODE_SWARM_ENABLED", "true");
        guard.set("JCODE_SWARM_MODEL", "");
        guard.set("JCODE_SWARM_EFFORT", "");
        guard.set("JCODE_SWARM_SPAWN_MODE", "headless");
        guard.set("JCODE_TOOL_PROFILE", "full");
        guard.set("JCODE_TOOLS", "*");
        guard.set("JCODE_DISABLED_TOOLS", "");
        guard.set("JCODE_DISABLE_BASE_TOOLS", "false");
        guard.set("JCODE_MCP_TOOLS", "deferred");
        guard.set("JCODE_MESSAGE_TIMESTAMPS", "false");
        guard.set("JCODE_CHECK_UPDATES", "false");
        guard.set("DO_NOT_TRACK", "1");
        guard
    }

    fn set_path(&mut self, key: &'static str, value: &Path) {
        self.saved.push((key, std::env::var_os(key)));
        crate::env::set_var(key, value);
    }

    fn set(&mut self, key: &'static str, value: &str) {
        self.saved.push((key, std::env::var_os(key)));
        crate::env::set_var(key, value);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.saved.drain(..).rev() {
            if let Some(previous) = previous {
                crate::env::set_var(key, previous);
            } else {
                crate::env::remove_var(key);
            }
        }
    }
}

fn init_git_repo(path: &Path) {
    let output = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .arg(path)
        .output()
        .expect("run git init");
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn swarm_member(
    session_id: &str,
    working_dir: PathBuf,
) -> (SwarmMember, mpsc::UnboundedReceiver<ServerEvent>) {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    (
        SwarmMember {
            session_id: session_id.to_string(),
            event_tx,
            event_txs: HashMap::new(),
            working_dir: Some(working_dir),
            swarm_id: Some(SWARM_ID.to_string()),
            swarm_enabled: true,
            status: "ready".to_string(),
            detail: None,
            task_label: None,
            friendly_name: Some("e1-captain".to_string()),
            report_back_to_session_id: None,
            latest_completion_report: None,
            role: "coordinator".to_string(),
            joined_at: Instant::now(),
            last_status_change: Instant::now(),
            is_headless: false,
            output_tail: None,
            todo_progress: None,
            todo_items: Vec::new(),
            runtime: crate::protocol::SwarmMemberRuntime::default(),
        },
        event_rx,
    )
}

struct E1Fixture {
    _root: tempfile::TempDir,
    _env: EnvGuard,
    home: PathBuf,
    nested: PathBuf,
    sessions: SessionAgents,
    global_session_id: Arc<RwLock<String>>,
    provider: Arc<dyn Provider>,
    captures: mpsc::UnboundedReceiver<CapturedRequest>,
    swarm_members: Arc<RwLock<HashMap<String, SwarmMember>>>,
    swarms_by_id: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    swarm_coordinators: Arc<RwLock<HashMap<String, String>>>,
    swarm_plans: Arc<RwLock<HashMap<String, VersionedPlan>>>,
    event_history: Arc<RwLock<VecDeque<SwarmEvent>>>,
    event_counter: Arc<AtomicU64>,
    swarm_event_tx: broadcast::Sender<SwarmEvent>,
    mcp_pool: Arc<crate::mcp::SharedMcpPool>,
    soft_interrupt_queues: SessionInterruptQueues,
    client_connections: ClientConnections,
}

impl E1Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().expect("create E1 fixture root");
        let home = root.path().join("home");
        let runtime = root.path().join("runtime");
        let repo = root.path().join("repo");
        let nested = repo.join("nested");
        std::fs::create_dir_all(home.join("external")).expect("create synthetic home");
        std::fs::create_dir_all(home.join("config")).expect("create synthetic config home");
        std::fs::create_dir_all(&runtime).expect("create runtime dir");
        std::fs::create_dir_all(&nested).expect("create nested repo dir");
        let env = EnvGuard::isolated(&home, &runtime);
        std::fs::write(home.join("external/AGENTS.md"), GLOBAL_POLICY)
            .expect("write global policy");
        std::fs::write(repo.join("AGENTS.md"), ROOT_POLICY).expect("write root policy");
        std::fs::write(nested.join("AGENTS.md"), NESTED_POLICY).expect("write nested policy");
        init_git_repo(&repo);
        assert!(!crate::config::config().features.memory);

        let (capture_tx, captures) = mpsc::unbounded_channel();
        let provider: Arc<dyn Provider> = Arc::new(CaptureProvider::new(capture_tx));
        let registry = Registry::new(Arc::clone(&provider)).await;
        let mut coordinator_session = crate::session::Session::create_with_id(
            COORDINATOR_ID.to_string(),
            None,
            Some(nested.to_string_lossy().to_string()),
        );
        coordinator_session.model = Some("gpt-5.6-sol".to_string());
        coordinator_session.provider_key = Some("openai".to_string());
        coordinator_session.route_api_method = Some("openai-oauth".to_string());
        coordinator_session.reasoning_effort = Some("high".to_string());
        coordinator_session.add_message(
            Role::User,
            vec![ContentBlock::Text {
                text: CAPTAIN_CANARY.to_string(),
                cache_control: None,
            }],
        );
        let coordinator = Arc::new(Mutex::new(Agent::new_with_session(
            Arc::clone(&provider),
            registry,
            coordinator_session,
            None,
        )));
        let sessions = Arc::new(RwLock::new(HashMap::from([(
            COORDINATOR_ID.to_string(),
            coordinator,
        )])));
        let (coordinator_member, _coordinator_events) =
            swarm_member(COORDINATOR_ID, nested.clone());
        let swarm_members = Arc::new(RwLock::new(HashMap::from([(
            COORDINATOR_ID.to_string(),
            coordinator_member,
        )])));
        let swarms_by_id = Arc::new(RwLock::new(HashMap::from([(
            SWARM_ID.to_string(),
            HashSet::from([COORDINATOR_ID.to_string()]),
        )])));
        let swarm_coordinators = Arc::new(RwLock::new(HashMap::from([(
            SWARM_ID.to_string(),
            COORDINATOR_ID.to_string(),
        )])));
        let (swarm_event_tx, _) = broadcast::channel(32);
        Self {
            _root: root,
            _env: env,
            home,
            nested,
            sessions,
            global_session_id: Arc::new(RwLock::new(COORDINATOR_ID.to_string())),
            provider,
            captures,
            swarm_members,
            swarms_by_id,
            swarm_coordinators,
            swarm_plans: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            event_counter: Arc::new(AtomicU64::new(0)),
            swarm_event_tx,
            mcp_pool: Arc::new(crate::mcp::SharedMcpPool::new(
                crate::mcp::McpConfig::default(),
            )),
            soft_interrupt_queues: Arc::new(RwLock::new(HashMap::new())),
            client_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn spawn_capture(
        &mut self,
        label: &str,
        packet: &str,
        allowed_tools: Option<&[&str]>,
    ) -> (String, CapturedRequest) {
        let session_id = spawn_swarm_agent(
            COORDINATOR_ID,
            SWARM_ID,
            Some(self.nested.to_string_lossy().to_string()),
            Some(packet.to_string()),
            Some(SwarmSpawnMode::Headless),
            Some("openai-oauth:gpt-5.6-sol".to_string()),
            Some("high".to_string()),
            Some(label.to_string()),
            allowed_tools.map(|tools| tools.iter().map(|tool| (*tool).to_string()).collect()),
            None,
            &self.sessions,
            &self.global_session_id,
            &self.provider,
            &self.swarm_members,
            &self.swarms_by_id,
            &self.swarm_coordinators,
            &self.swarm_plans,
            &self.event_history,
            &self.event_counter,
            &self.swarm_event_tx,
            &self.mcp_pool,
            &self.soft_interrupt_queues,
            &self.client_connections,
        )
        .await
        .expect("spawn synthetic worker");
        let captured = tokio::time::timeout(Duration::from_secs(2), self.captures.recv())
            .await
            .expect("capture timeout")
            .expect("captured request");
        self.wait_until_ready(&session_id).await;
        (session_id, captured)
    }

    async fn capture_existing_turn(&mut self, session_id: &str, packet: &str) -> CapturedRequest {
        let worker = self
            .sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .expect("spawned worker session");
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        process_message_streaming_mpsc(worker, packet, vec![], None, event_tx)
            .await
            .expect("run existing synthetic worker turn");
        tokio::time::timeout(Duration::from_secs(2), self.captures.recv())
            .await
            .expect("capture timeout")
            .expect("captured request")
    }

    async fn wait_until_ready(&self, session_id: &str) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if self
                    .swarm_members
                    .read()
                    .await
                    .get(session_id)
                    .is_some_and(|member| member.status == "ready")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("worker did not return to ready");
    }
}

fn assert_required_context(captured: &CapturedRequest, packet_marker: &str, nested_policy: &str) {
    assert!(captured.system_static.contains(GLOBAL_POLICY));
    assert!(captured.system_static.contains(ROOT_POLICY));
    assert!(captured.system_static.contains(nested_policy));
    assert!(!captured.system_static.contains(CAPTAIN_CANARY));
    let messages = captured.rendered_messages();
    assert!(messages.contains(packet_marker));
    assert!(messages.contains("SWARM COMPLETION REPORT REQUIRED"));
    assert!(!messages.contains(CAPTAIN_CANARY));
}

fn median(values: &mut [usize]) -> usize {
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2
    } else {
        values[middle]
    }
}

#[tokio::test]
async fn worker_context_receipt_real_spawn_excludes_parent_canary_and_keeps_required_policy() {
    let _env_lock = crate::storage::lock_test_env();
    let mut fixture = E1Fixture::new().await;
    let (_session_id, captured) = fixture
        .spawn_capture(
            "policy proof",
            "E1_PACKET_POLICY_PROOF inspect one synthetic file",
            Some(&["read", "agentgrep"]),
        )
        .await;

    assert_required_context(&captured, "E1_PACKET_POLICY_PROOF", NESTED_POLICY);
    assert_eq!(
        captured
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["agentgrep", "read"]
    );
}

#[tokio::test]
async fn worker_context_receipt_repeats_stable_prefix_and_measures_existing_tool_narrowing() {
    let _env_lock = crate::storage::lock_test_env();
    let mut fixture = E1Fixture::new().await;
    let tasks: [(&str, &str, &[&str]); 3] = [
        (
            "read-only-map",
            "E1_TASK_READ_ONLY_MAP map two named symbols and report paths only",
            &["agentgrep", "read"],
        ),
        (
            "mechanical-doc-edit",
            "E1_TASK_MECHANICAL_DOC_EDIT make one prescribed documentation replacement",
            &["edit", "read"],
        ),
        (
            "targeted-test-fix",
            "E1_TASK_TARGETED_TEST_FIX inspect one named test failure and propose a narrow fix",
            &["agentgrep", "bash", "read"],
        ),
    ];
    let mut narrowed_by_task = Vec::new();
    let mut inherited_by_task = Vec::new();
    let mut narrowed_totals = Vec::new();
    let mut inherited_totals = Vec::new();

    for (label, packet, tools) in tasks {
        let mut narrowed_repeats = Vec::new();
        let mut inherited_repeats = Vec::new();
        for repeat in 1..=2 {
            let (_, narrowed) = fixture.spawn_capture(label, packet, Some(tools)).await;
            assert_required_context(
                &narrowed,
                packet.split_whitespace().next().unwrap(),
                NESTED_POLICY,
            );
            let narrowed_receipt = narrowed.receipt(label, repeat, "explicit-relevant-tools");

            let (_, inherited) = fixture.spawn_capture(label, packet, None).await;
            assert_required_context(
                &inherited,
                packet.split_whitespace().next().unwrap(),
                NESTED_POLICY,
            );
            let inherited_receipt = inherited.receipt(label, repeat, "inherited-tools");
            assert!(
                narrowed_receipt.captured_serialized_bytes
                    < inherited_receipt.captured_serialized_bytes,
                "explicit relevant tools should reduce this captured request"
            );

            eprintln!(
                "E1_CONTEXT_CAPTURE {}",
                serde_json::to_string(&narrowed_receipt).expect("serialize narrowed receipt")
            );
            eprintln!(
                "E1_CONTEXT_CAPTURE {}",
                serde_json::to_string(&inherited_receipt).expect("serialize inherited receipt")
            );
            narrowed_totals.push(narrowed_receipt.captured_serialized_bytes);
            inherited_totals.push(inherited_receipt.captured_serialized_bytes);
            narrowed_repeats.push(narrowed_receipt);
            inherited_repeats.push(inherited_receipt);
        }
        narrowed_by_task.push(narrowed_repeats);
        inherited_by_task.push(inherited_repeats);
    }

    for repeats in narrowed_by_task.iter().chain(&inherited_by_task) {
        assert_eq!(repeats[0].system_static_hash, repeats[1].system_static_hash);
        assert_eq!(repeats[0].tools_hash, repeats[1].tools_hash);
        assert_eq!(repeats[0].stable_prefix_hash, repeats[1].stable_prefix_hash);
        assert_eq!(
            repeats[0].system_static_bytes,
            repeats[1].system_static_bytes
        );
        assert_eq!(repeats[0].tools_json_bytes, repeats[1].tools_json_bytes);
    }
    assert_ne!(
        narrowed_by_task[0][0].tools_hash,
        narrowed_by_task[1][0].tools_hash
    );
    assert_ne!(
        narrowed_by_task[0][0].stable_prefix_hash,
        narrowed_by_task[1][0].stable_prefix_hash
    );

    let narrowed_median = median(&mut narrowed_totals);
    let inherited_median = median(&mut inherited_totals);
    eprintln!(
        "E1_EXISTING_CAPABILITY_SUMMARY inherited_median={} explicit_relevant_median={} captured_serialized_savings={} note=provider-bound-high-level-serialization-not-wire-bytes",
        inherited_median,
        narrowed_median,
        inherited_median.saturating_sub(narrowed_median),
    );
}

#[tokio::test]
async fn worker_context_receipt_real_registry_schema_change_invalidates_tool_prefix() {
    let _env_lock = crate::storage::lock_test_env();
    let mut fixture = E1Fixture::new().await;
    let swarm_prompt = fixture.home.join("swarm-prompt.md");
    std::fs::write(&swarm_prompt, "E1_SWARM_SCHEMA_REVISION_ONE")
        .expect("write first synthetic swarm schema input");
    let (_, first) = fixture
        .spawn_capture(
            "schema revision one",
            "E1_SCHEMA_PACKET inspect the synthetic schema snapshot",
            Some(&["swarm"]),
        )
        .await;
    std::fs::write(&swarm_prompt, "E1_SWARM_SCHEMA_REVISION_TWO")
        .expect("write second synthetic swarm schema input");
    let (_, second) = fixture
        .spawn_capture(
            "schema revision two",
            "E1_SCHEMA_PACKET inspect the synthetic schema snapshot",
            Some(&["swarm"]),
        )
        .await;

    let first_receipt = first.receipt("schema revision one", 1, "explicit-relevant-tools");
    let second_receipt = second.receipt("schema revision two", 1, "explicit-relevant-tools");
    assert_eq!(first_receipt.tool_names, vec!["swarm"]);
    assert_eq!(second_receipt.tool_names, vec!["swarm"]);
    assert_eq!(
        first_receipt.system_static_hash,
        second_receipt.system_static_hash
    );
    assert_ne!(first_receipt.tools_hash, second_receipt.tools_hash);
    assert_ne!(
        first_receipt.stable_prefix_hash,
        second_receipt.stable_prefix_hash
    );
    assert!(
        first.tools[0]
            .description
            .contains("E1_SWARM_SCHEMA_REVISION_ONE")
    );
    assert!(
        second.tools[0]
            .description
            .contains("E1_SWARM_SCHEMA_REVISION_TWO")
    );
}

#[tokio::test]
async fn worker_context_receipt_policy_snapshot_stays_stable_then_fresh_spawn_invalidates() {
    let _env_lock = crate::storage::lock_test_env();
    let mut fixture = E1Fixture::new().await;
    let (session_id, before) = fixture
        .spawn_capture(
            "policy snapshot",
            "E1_POLICY_FIRST_PACKET capture the initial policy snapshot",
            Some(&["agentgrep", "read"]),
        )
        .await;
    std::fs::write(fixture.nested.join("AGENTS.md"), NESTED_POLICY_REVISION)
        .expect("revise synthetic policy");

    let same_agent = fixture
        .capture_existing_turn(
            &session_id,
            "E1_POLICY_SAME_AGENT_PACKET retain the captured policy snapshot",
        )
        .await;
    assert_required_context(&same_agent, "E1_POLICY_SAME_AGENT_PACKET", NESTED_POLICY);
    assert!(!same_agent.system_static.contains(NESTED_POLICY_REVISION));
    assert_eq!(
        before
            .receipt("before policy edit", 1, "explicit-relevant-tools")
            .system_static_hash,
        same_agent
            .receipt("same agent after policy edit", 1, "explicit-relevant-tools")
            .system_static_hash
    );

    let (_, fresh_agent) = fixture
        .spawn_capture(
            "fresh policy snapshot",
            "E1_POLICY_FRESH_AGENT_PACKET capture the revised policy snapshot",
            Some(&["agentgrep", "read"]),
        )
        .await;
    assert_required_context(
        &fresh_agent,
        "E1_POLICY_FRESH_AGENT_PACKET",
        NESTED_POLICY_REVISION,
    );
    assert!(!fresh_agent.system_static.contains(NESTED_POLICY));
    assert_ne!(
        before
            .receipt("before policy edit", 1, "explicit-relevant-tools")
            .system_static_hash,
        fresh_agent
            .receipt(
                "fresh agent after policy edit",
                1,
                "explicit-relevant-tools"
            )
            .system_static_hash
    );
}

#[tokio::test]
async fn worker_context_receipt_unsupported_cache_metrics_stay_unknown() {
    let _env_lock = crate::storage::lock_test_env();
    let mut fixture = E1Fixture::new().await;
    let (session_id, _captured) = fixture
        .spawn_capture(
            "cache unknown",
            "E1_PACKET_CACHE_UNKNOWN inspect synthetic usage",
            Some(&["read"]),
        )
        .await;
    let worker = fixture
        .sessions
        .read()
        .await
        .get(&session_id)
        .cloned()
        .expect("worker session");
    let usage = worker.lock().await.last_usage().clone();
    assert_eq!(usage.input_tokens, 321);
    assert_eq!(usage.output_tokens, 12);
    assert_eq!(usage.cache_read_input_tokens, None);
    assert_eq!(usage.cache_creation_input_tokens, None);
}
