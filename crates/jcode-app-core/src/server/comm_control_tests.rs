use super::{handle_comm_assign_next, handle_comm_assign_task, handle_comm_task_control};
use crate::agent::Agent;
use crate::message::{Message, StreamEvent, ToolDefinition};
use crate::plan::PlanItem;
use crate::protocol::ServerEvent;
use crate::provider::{EventStream, Provider};
use crate::server::comm_await::{CommAwaitMembersContext, handle_comm_await_members};
use crate::server::{
    AwaitMembersRuntime, SwarmEvent, SwarmEventType, SwarmMember, SwarmMutationRuntime,
    VersionedPlan,
};
use crate::tool::Registry;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};

struct RuntimeEnvGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
    prev_runtime: Option<std::ffi::OsString>,
}

impl RuntimeEnvGuard {
    fn new() -> (Self, tempfile::TempDir) {
        let guard = crate::storage::lock_test_env();
        let temp = tempfile::TempDir::new().expect("create runtime dir");
        let prev_runtime = std::env::var_os("JCODE_RUNTIME_DIR");
        crate::env::set_var("JCODE_RUNTIME_DIR", temp.path());
        (
            Self {
                _guard: guard,
                prev_runtime,
            },
            temp,
        )
    }
}

impl Drop for RuntimeEnvGuard {
    fn drop(&mut self) {
        if let Some(prev_runtime) = self.prev_runtime.take() {
            crate::env::set_var("JCODE_RUNTIME_DIR", prev_runtime);
        } else {
            crate::env::remove_var("JCODE_RUNTIME_DIR");
        }
    }
}

fn member(session_id: &str, swarm_id: &str, status: &str) -> SwarmMember {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    SwarmMember {
        session_id: session_id.to_string(),
        event_tx,
        event_txs: HashMap::new(),
        working_dir: None,
        swarm_id: Some(swarm_id.to_string()),
        swarm_enabled: true,
        status: status.to_string(),
        detail: None,
        friendly_name: Some(session_id.to_string()),
        report_back_to_session_id: None,
        latest_completion_report: None,
        role: "agent".to_string(),
        joined_at: Instant::now(),
        last_status_change: Instant::now(),
        is_headless: false,
        output_tail: None,
        todo_progress: None,
        todo_items: Vec::new(),
        runtime: crate::protocol::SwarmMemberRuntime::default(),
        task_label: None,
    }
}

/// A swarm worker owned by `owner` (its spawning coordinator). Auto-assignment
/// only targets such drivable workers, so test fixtures that model a spawned
/// worker should use this rather than a bare `member()` (which represents a
/// foreign/independent session and is intentionally not auto-assignable).
fn owned_member(session_id: &str, swarm_id: &str, status: &str, owner: &str) -> SwarmMember {
    let mut m = member(session_id, swarm_id, status);
    m.report_back_to_session_id = Some(owner.to_string());
    m
}

fn plan_item(id: &str, status: &str, priority: &str, blocked_by: &[&str]) -> PlanItem {
    PlanItem {
        content: format!("task {id}"),
        status: status.to_string(),
        priority: priority.to_string(),
        id: id.to_string(),
        subsystem: None,
        file_scope: Vec::new(),
        blocked_by: blocked_by.iter().map(|value| value.to_string()).collect(),
        assigned_to: None,
    }
}

fn swarm_event(session_id: &str, swarm_id: &str, event: SwarmEventType) -> SwarmEvent {
    SwarmEvent {
        id: 1,
        session_id: session_id.to_string(),
        session_name: Some(session_id.to_string()),
        swarm_id: Some(swarm_id.to_string()),
        event,
        timestamp: Instant::now(),
        absolute_time: SystemTime::now(),
    }
}

#[derive(Default)]
struct TestProvider;

#[async_trait]
impl Provider for TestProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        Ok(Box::pin(stream::iter(vec![Ok(StreamEvent::MessageEnd {
            stop_reason: None,
        })])))
    }

    fn name(&self) -> &str {
        "test"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self)
    }
}

async fn test_agent() -> Arc<Mutex<Agent>> {
    let provider: Arc<dyn Provider> = Arc::new(TestProvider);
    let registry = Registry::new(provider.clone()).await;
    Arc::new(Mutex::new(Agent::new(provider, registry)))
}

struct DetachedTaskEnvGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Drop for DetachedTaskEnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.0.drain(..) {
            match previous {
                Some(value) => crate::env::set_var(key, value),
                None => crate::env::remove_var(key),
            }
        }
    }
}

#[derive(Clone)]
struct DetachedTaskCountingProvider(Arc<AtomicU64>);

#[async_trait]
impl Provider for DetachedTaskCountingProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        system: &str,
        resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        TestProvider
            .complete(messages, tools, system, resume_session_id)
            .await
    }

    fn name(&self) -> &str {
        "test"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

fn check_swarm_task_dispatch(case: &str, assign: bool) {
    check_swarm_task_dispatch_with_ack(case, assign, "none");
}

fn check_swarm_task_dispatch_with_ack(case: &str, assign: bool, ack_mode: &str) {
    let (_env, temp) = RuntimeEnvGuard::new();
    let _isolated_env = DetachedTaskEnvGuard(
        ["JCODE_HOME", "JCODE_NO_TELEMETRY", "JCODE_HOOKS_DISABLED"]
            .map(|key| (key, std::env::var_os(key)))
            .into(),
    );
    crate::env::set_var("JCODE_HOME", temp.path().join("home"));
    crate::env::set_var("JCODE_NO_TELEMETRY", "1");
    crate::env::set_var("JCODE_HOOKS_DISABLED", "1");
    // Drop this private runtime (including any progress pump) before restoring
    // the environment, on both success and assertion failure.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let swarm_id = "detached-current-owner";
        let worker = "worker";
        let calls = Arc::new(AtomicU64::new(0));
        let provider = Arc::new(DetachedTaskCountingProvider(Arc::clone(&calls)));
        let mut agent =
            Agent::new_with_initial_working_dir(provider, Registry::empty(), temp.path().to_str());
        agent.set_memory_enabled(false);
        let agent = Arc::new(Mutex::new(agent));
        let (member_tx, mut member_rx) = mpsc::unbounded_channel();
        let mut target = if assign {
            owned_member(worker, swarm_id, "ready", "coord")
        } else {
            member(worker, swarm_id, "ready")
        };
        target.event_tx = member_tx.clone();
        let members = Arc::new(RwLock::new(HashMap::from([(worker.to_string(), target)])));
        let swarms = Arc::new(RwLock::new(HashMap::from([(
            swarm_id.to_string(),
            HashSet::from([worker.to_string()]),
        )])));
        let coordinators = Arc::new(RwLock::new(HashMap::from([(
            swarm_id.to_string(),
            "coord".to_string(),
        )])));
        let mut item = plan_item("task", "queued", "high", &[]);
        if !assign {
            item.assigned_to = Some(worker.to_string());
        }
        let mut plan = VersionedPlan::new();
        plan.items.push(item);
        plan.version = 7;
        plan.participants.insert(worker.to_string());
        if !assign {
            plan.task_progress
                .entry("task".to_string())
                .or_default()
                .assigned_session_id = Some(worker.to_string());
        }
        if assign {
            let mut coordinator = member("coord", swarm_id, "ready");
            coordinator.role = "coordinator".to_string();
            coordinator.event_tx = member_tx;
            members
                .write()
                .await
                .insert("coord".to_string(), coordinator);
            swarms
                .write()
                .await
                .get_mut(swarm_id)
                .unwrap()
                .insert("coord".to_string());
        }
        let plans = Arc::new(RwLock::new(HashMap::from([(swarm_id.to_string(), plan)])));
        let history = Arc::new(RwLock::new(VecDeque::new()));
        let counter = Arc::new(AtomicU64::new(7));
        let (event_tx, mut event_rx) = broadcast::channel(64);
        let snapshot = crate::storage::durable_state_dir()
            .join("swarm")
            .join(format!("{swarm_id}.json"));
        assert!(!snapshot.exists());
        if case == "save_error" {
            // Break only the actual swarm snapshot parent. Mutation admission
            // still writes to its separate runtime ledger directory.
            std::fs::create_dir_all(snapshot.parent().unwrap().parent().unwrap()).unwrap();
            std::fs::write(snapshot.parent().unwrap(), b"not a directory").unwrap();
        } else if case == "save_stale" {
            let mut newer = plans.read().await[swarm_id].clone();
            newer.version = 100;
            crate::server::swarm_persistence::persist_swarm_state(
                swarm_id,
                Some(&newer),
                Some("coord"),
                &[],
            );
            let saved: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&snapshot).unwrap()).unwrap();
            assert_eq!(saved["plan"]["version"], 100);
        }
        let before_snapshot = std::fs::read(&snapshot).ok();

        if assign {
            let before_plan = format!("{:?}", *plans.read().await);
            let before_members = format!("{:?}", *members.read().await);
            let before_swarms = swarms.read().await.clone();
            let before_coordinators = coordinators.read().await.clone();
            let sessions = Arc::new(RwLock::new(HashMap::from([(
                worker.to_string(),
                Arc::clone(&agent),
            )])));
            let interrupts = Arc::new(RwLock::new(HashMap::new()));
            let (disconnect_tx, _disconnect_rx) = mpsc::unbounded_channel();
            let connections = Arc::new(RwLock::new(HashMap::from([(
                "conn".to_string(),
                crate::server::ClientConnectionInfo {
                    client_id: "conn".to_string(),
                    session_id: worker.to_string(),
                    client_instance_id: None,
                    debug_client_id: None,
                    connected_at: Instant::now(),
                    last_seen: Instant::now(),
                    is_processing: false,
                    current_tool_name: None,
                    terminal_env: Vec::new(),
                    supports_user_response: false,
                    disconnect_tx,
                },
            )])));
            let (client_tx, mut client_rx) = mpsc::unbounded_channel();
            let mutations = SwarmMutationRuntime::default();
            let key = super::swarm_mutation_request_key(
                "coord",
                "assign_task",
                &[
                    swarm_id.to_string(),
                    "__next_available__".to_string(),
                    "task".to_string(),
                    String::new(),
                ],
            );
            let ledger = crate::server::durable_state::state_path("jcode-swarm-mutations", &key);
            let assignment = handle_comm_assign_task(
                91,
                "coord".to_string(),
                None,
                Some("task".to_string()),
                None,
                &client_tx,
                &sessions,
                &interrupts,
                &connections,
                &members,
                &swarms,
                &plans,
                &coordinators,
                &history,
                &counter,
                &event_tx,
                &mutations,
            );
            tokio::pin!(assignment);
            // A read guard permits authorization/selection but stops the real
            // consumer's plan write. One poll reaches that deterministic gate,
            // after admission, without a spawned task, sleep, or polling loop.
            let plan_guard = plans.read().await;
            assert!(futures::poll!(assignment.as_mut()).is_pending());
            let pending: crate::server::swarm_mutation_state::PersistedSwarmMutationState =
                serde_json::from_slice(&std::fs::read(&ledger).unwrap()).unwrap();
            assert!(pending.final_response.is_none(), "admission must succeed");
            assert!(
                super::begin_swarm_mutation_or_replay(
                    &mutations,
                    &key,
                    "assign_task",
                    "coord",
                    92,
                    &client_tx,
                )
                .await
                .is_none(),
                "duplicate must join the admitted assignment"
            );
            drop(plan_guard);
            tokio::time::timeout(Duration::from_secs(10), assignment.as_mut())
                .await
                .expect("assignment handler timed out");

            assert!(
                !super::auto_assign_claims()
                    .lock()
                    .unwrap()
                    .contains_key(&super::auto_assign_claim_key(swarm_id, worker)),
                "assignment must release its auto-pick claim"
            );
            let final_state: crate::server::swarm_mutation_state::PersistedSwarmMutationState =
                serde_json::from_slice(&std::fs::read(&ledger).unwrap()).unwrap();
            let valid = case == "save_valid";
            for expected_id in [91, 92] {
                let response = client_rx
                    .try_recv()
                    .expect("every mutation waiter must finish");
                match (valid, response) {
                    (
                        true,
                        ServerEvent::CommAssignTaskResponse {
                            id,
                            task_id,
                            target_session,
                        },
                    ) => {
                        assert_eq!(id, expected_id);
                        assert_eq!(task_id, "task");
                        assert_eq!(target_session, worker);
                    }
                    (false, ServerEvent::Error { id, .. }) => assert_eq!(id, expected_id),
                    (_, other) => panic!("unexpected assignment response for {case}: {other:?}"),
                }
            }
            assert!(matches!(
                client_rx.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
            if valid {
                assert!(matches!(
                    final_state.final_response,
                    Some(super::PersistedSwarmMutationResponse::AssignTask { .. })
                ));
                let locked = plans.read().await;
                assert_eq!(locked[swarm_id].version, 8);
                assert_eq!(locked[swarm_id].items[0].status, "queued");
                assert_eq!(
                    locked[swarm_id].items[0].assigned_to.as_deref(),
                    Some(worker)
                );
                assert!(agent.lock().await.has_soft_interrupts());
                assert!(!history.read().await.is_empty());
                assert!(member_rx.try_recv().is_ok());
                assert!(event_rx.try_recv().is_ok());
                let saved: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&snapshot).unwrap()).unwrap();
                assert_eq!(saved["plan"]["version"], 8);
                assert_eq!(saved["plan"]["items"][0]["assigned_to"], worker);
            } else {
                assert!(matches!(
                    final_state.final_response,
                    Some(super::PersistedSwarmMutationResponse::Error { .. })
                ));
                assert_eq!(format!("{:?}", *plans.read().await), before_plan);
                assert_eq!(format!("{:?}", *members.read().await), before_members);
                assert_eq!(*swarms.read().await, before_swarms);
                assert_eq!(*coordinators.read().await, before_coordinators);
                assert!(!agent.lock().await.has_soft_interrupts());
                assert!(interrupts.read().await.is_empty());
                assert!(history.read().await.is_empty());
                assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 7);
                assert!(matches!(
                    member_rx.try_recv(),
                    Err(mpsc::error::TryRecvError::Empty)
                ));
                assert!(matches!(
                    event_rx.try_recv(),
                    Err(broadcast::error::TryRecvError::Empty)
                ));
                assert_eq!(std::fs::read(&snapshot).ok(), before_snapshot);
            }
            return;
        }

        if case.starts_with("requeue_") {
            if case == "requeue_stale" {
                plans.write().await.get_mut(swarm_id).unwrap().items[0].status =
                    "running_stale".to_string();
            }
            let selected = super::task_snapshot_for(swarm_id, "task", &plans)
                .await
                .unwrap();
            {
                let mut locked = plans.write().await;
                match case {
                    "requeue_stale" => {}
                    "requeue_missing_plan" => {
                        locked.remove(swarm_id);
                    }
                    "requeue_missing_task" => locked.get_mut(swarm_id).unwrap().items.clear(),
                    _ => unreachable!(),
                }
            }
            let requeued = super::requeue_existing_assignment(
                swarm_id,
                "coord",
                worker,
                "task",
                "inert task".to_string(),
                &selected,
                &super::SwarmState {
                    members: Arc::clone(&members),
                    swarms_by_id: Arc::clone(&swarms),
                    plans: Arc::clone(&plans),
                    coordinators: Arc::clone(&coordinators),
                },
            )
            .await;
            assert_eq!(requeued.is_ok(), case == "requeue_stale");
            if requeued.is_ok() {
                assert_eq!(plans.read().await[swarm_id].items[0].status, "queued");
            }
        }

        let (startup_tx, startup_rx) = tokio::sync::oneshot::channel();
        let startup_ack = (ack_mode != "none").then_some(startup_tx);
        let startup_rx = if ack_mode == "drop" {
            drop(startup_rx);
            None
        } else {
            Some(startup_rx)
        };
        // Queue the real consumer behind the write lock, then displace its
        // assignment before it can observe the authoritative current state.
        let mut locked = plans.write().await;
        let mut handle = super::spawn_assigned_task_run(
            agent,
            worker.to_string(),
            swarm_id.to_string(),
            "task".to_string(),
            "inert task".to_string(),
            Arc::clone(&members),
            Arc::clone(&swarms),
            Arc::clone(&plans),
            Arc::clone(&coordinators),
            Arc::clone(&history),
            Arc::clone(&counter),
            event_tx,
            None,
            startup_ack,
        );
        match case {
            "missing_plan" => {
                locked.remove(swarm_id);
            }
            "requeue_missing_plan" | "requeue_missing_task" => {}
            _ => {
                let plan = locked.get_mut(swarm_id).unwrap();
                match case {
                    "missing_task" => plan.items.clear(),
                    "item_owner" => plan.items[0].assigned_to = Some("replacement".to_string()),
                    "progress_owner" => {
                        plan.task_progress
                            .get_mut("task")
                            .unwrap()
                            .assigned_session_id = Some("replacement".to_string());
                    }
                    "both_owners" => {
                        plan.items[0].assigned_to = Some("replacement".to_string());
                        plan.task_progress
                            .get_mut("task")
                            .unwrap()
                            .assigned_session_id = Some("replacement".to_string());
                    }
                    "absent_item_owner" => plan.items[0].assigned_to = None,
                    "absent_progress_owner" => {
                        plan.task_progress
                            .get_mut("task")
                            .unwrap()
                            .assigned_session_id = None;
                    }
                    "absent_progress" => {
                        plan.task_progress.remove("task");
                    }
                    "queued" | "requeue_stale" | "save_error" | "save_stale" | "save_valid" => {}
                    status => plan.items[0].status = status.to_string(),
                }
            }
        }
        let before_plan = format!("{:?}", *locked);
        let before_members = format!("{:?}", *members.read().await);
        let before_swarms = swarms.read().await.clone();
        let before_coordinators = coordinators.read().await.clone();
        drop(locked);

        match tokio::time::timeout(Duration::from_secs(10), &mut handle).await {
            Ok(result) => result.expect("detached runner did not panic"),
            Err(_) => {
                handle.abort();
                let _ = handle.await;
                panic!("detached runner timed out: {case}");
            }
        }
        if ack_mode == "reject" {
            let admission = startup_rx
                .unwrap()
                .await
                .expect("runner sends explicit rejection");
            assert!(
                admission.is_err(),
                "displaced owner must receive rejected startup"
            );
        } else if ack_mode == "drop" {
            assert_eq!(plans.read().await[swarm_id].items[0].status, "done");
            assert_eq!(members.read().await[worker].status, "completed");
        }
        let calls = calls.load(std::sync::atomic::Ordering::SeqCst);
        if matches!(case, "queued" | "requeue_stale" | "save_valid") {
            assert!(
                calls > 0,
                "valid queued owner must reach the inert provider"
            );
            if case == "save_valid" {
                assert!(
                    snapshot.is_file(),
                    "valid startup must save a swarm snapshot"
                );
            }
        } else {
            assert_eq!(calls, 0, "rejected startup reached provider: {case}");
            assert_eq!(format!("{:?}", *plans.read().await), before_plan);
            assert_eq!(format!("{:?}", *members.read().await), before_members);
            assert_eq!(*swarms.read().await, before_swarms);
            assert_eq!(*coordinators.read().await, before_coordinators);
            assert!(history.read().await.is_empty());
            assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 7);
            assert!(matches!(
                member_rx.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            assert!(matches!(
                event_rx.try_recv(),
                Err(broadcast::error::TryRecvError::Closed)
            ));
            assert_eq!(
                std::fs::read(&snapshot).ok(),
                before_snapshot,
                "rejection must preserve the existing swarm snapshot"
            );
            if case != "save_stale" {
                assert!(!snapshot.exists(), "rejection persisted a swarm snapshot");
            }
        }
    });
}

#[test]
fn requeue_start_admission_dropped_ack_receiver_keeps_execution() {
    check_swarm_task_dispatch_with_ack("queued", false, "drop");
}

#[test]
fn requeue_start_admission_rejected_ack_is_consumed() {
    check_swarm_task_dispatch_with_ack("item_owner", false, "reject");
}

#[tokio::test]
async fn requeue_start_admission_closed_ack_is_error() {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    drop(sender);
    let error = super::task_run_startup_result(receiver).await.unwrap_err();
    assert!(error.contains("without an admission acknowledgment"));
}

macro_rules! detached_task_start_current_owner_cases {
    ($($name:ident => $case:literal),+ $(,)?) => {$(
        #[test]
        fn $name() {
            check_swarm_task_dispatch($case, false);
        }
    )+};
}

detached_task_start_current_owner_cases! {
    detached_task_start_current_owner_missing_plan => "missing_plan",
    detached_task_start_current_owner_missing_task => "missing_task",
    detached_task_start_current_owner_item_owner => "item_owner",
    detached_task_start_current_owner_progress_owner => "progress_owner",
    detached_task_start_current_owner_both_owners => "both_owners",
    detached_task_start_current_owner_absent_item_owner => "absent_item_owner",
    detached_task_start_current_owner_absent_progress_owner => "absent_progress_owner",
    detached_task_start_current_owner_absent_progress => "absent_progress",
    detached_task_start_current_owner_running_duplicate => "running",
    detached_task_start_current_owner_direct_stale => "running_stale",
    detached_task_start_current_owner_done => "done",
    detached_task_start_current_owner_failed => "failed",
    detached_task_start_current_owner_cancelled => "cancelled",
    detached_task_start_current_owner_pending => "pending",
    detached_task_start_current_owner_blocked => "blocked",
    detached_task_start_current_owner_requeue_missing_plan => "requeue_missing_plan",
    detached_task_start_current_owner_requeue_missing_task => "requeue_missing_task",
    detached_task_start_current_owner_queued => "queued",
    detached_task_start_current_owner_requeue_stale => "requeue_stale",
}

macro_rules! checked_swarm_save_gate_cases {
    ($($name:ident => ($case:literal, $assign:literal)),+ $(,)?) => {$(
        #[test]
        fn $name() {
            check_swarm_task_dispatch($case, $assign);
        }
    )+};
}

checked_swarm_save_gate_cases! {
    checked_swarm_save_gate_assignment_io_error => ("save_error", true),
    checked_swarm_save_gate_assignment_newer_disk_plan => ("save_stale", true),
    checked_swarm_save_gate_assignment_valid => ("save_valid", true),
    checked_swarm_save_gate_detached_io_error => ("save_error", false),
    checked_swarm_save_gate_detached_newer_disk_plan => ("save_stale", false),
    checked_swarm_save_gate_detached_valid => ("save_valid", false),
}

#[derive(Clone)]
struct TerminalBarrierProvider {
    calls: Arc<AtomicU64>,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    fail: bool,
}

#[async_trait]
impl Provider for TerminalBarrierProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.entered.notify_one();
        self.release.notified().await;
        if self.fail {
            anyhow::bail!("inert terminal failure");
        }
        Ok(Box::pin(stream::iter(vec![
            Ok(StreamEvent::TextDelta("inert actor report".to_string())),
            Ok(StreamEvent::MessageEnd { stop_reason: None }),
        ])))
    }

    fn name(&self) -> &str {
        "test"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

fn check_current_owner_terminal(case: &str, fail: bool) {
    let (_env, temp) = RuntimeEnvGuard::new();
    let _isolated_env = DetachedTaskEnvGuard(
        ["JCODE_HOME", "JCODE_NO_TELEMETRY", "JCODE_HOOKS_DISABLED"]
            .map(|key| (key, std::env::var_os(key)))
            .into(),
    );
    crate::env::set_var("JCODE_HOME", temp.path().join("home"));
    crate::env::set_var("JCODE_NO_TELEMETRY", "1");
    crate::env::set_var("JCODE_HOOKS_DISABLED", "1");
    // No wall-clock sleeps or heartbeat races. The bounded waits below also
    // clean up the actual runner before this isolated runtime/environment drops.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap();
    runtime.block_on(async {
        let swarm_id = "terminal-current-owner";
        let worker = "worker";
        let calls = Arc::new(AtomicU64::new(0));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let provider = Arc::new(TerminalBarrierProvider {
            calls: Arc::clone(&calls),
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
            fail,
        });
        let mut agent =
            Agent::new_with_initial_working_dir(provider, Registry::empty(), temp.path().to_str());
        agent.set_memory_enabled(false);
        let agent = Arc::new(Mutex::new(agent));
        let (member_tx, mut member_rx) = mpsc::unbounded_channel();
        let mut actor = owned_member(worker, swarm_id, "ready", "coord");
        actor.event_tx = member_tx.clone();
        let mut coord = member("coord", swarm_id, "ready");
        coord.role = "coordinator".to_string();
        coord.event_tx = member_tx;
        let members = Arc::new(RwLock::new(HashMap::from([
            (worker.to_string(), actor),
            ("coord".to_string(), coord),
        ])));
        let swarms = Arc::new(RwLock::new(HashMap::from([(
            swarm_id.to_string(),
            HashSet::from([worker.to_string(), "coord".to_string()]),
        )])));
        let coordinators = Arc::new(RwLock::new(HashMap::from([(
            swarm_id.to_string(),
            "coord".to_string(),
        )])));
        let mut plan = VersionedPlan::new();
        plan.version = 7;
        plan.mode = if case.contains("deep") || case == "complete_node" {
            "deep"
        } else {
            "light"
        }
        .to_string();
        let mut item = plan_item("task", "queued", "high", &[]);
        item.assigned_to = Some(worker.to_string());
        plan.items.push(item);
        // A dependent makes the light success broadcast's newly-ready payload
        // observable, while the unrelated artifact must survive every case.
        plan.items.push(plan_item("dependent", "queued", "low", &["task"]));
        plan.items.push(plan_item("unrelated", "done", "low", &[]));
        plan.node_meta.insert("unrelated".to_string(), crate::plan::NodeMeta {
            artifact_json: Some(serde_json::json!({"findings": "unrelated evidence"}).to_string()),
            ..Default::default()
        });
        plan.participants.insert(worker.to_string());
        let progress = plan.task_progress.entry("task".to_string()).or_default();
        progress.assigned_session_id = Some(worker.to_string());
        progress.assigned_at_unix_ms = Some(111);
        progress.dead_assignee_reclaims = Some(2);
        if case.contains("repeated") {
            progress.no_artifact_requeues = Some(1);
        }
        let plans = Arc::new(RwLock::new(HashMap::from([(swarm_id.to_string(), plan)])));
        let history = Arc::new(RwLock::new(VecDeque::new()));
        let counter = Arc::new(AtomicU64::new(7));
        let (event_tx, mut event_rx) = broadcast::channel(64);
        let mut handle = super::spawn_assigned_task_run(
            agent,
            worker.to_string(),
            swarm_id.to_string(),
            "task".to_string(),
            "inert task".to_string(),
            Arc::clone(&members),
            Arc::clone(&swarms),
            Arc::clone(&plans),
            Arc::clone(&coordinators),
            Arc::clone(&history),
            Arc::clone(&counter),
            event_tx.clone(),
            None,
            None,
        );
        if tokio::time::timeout(Duration::from_secs(10), entered.notified()).await.is_err() {
            handle.abort();
            let _ = handle.await;
            panic!("provider barrier timed out: {case}");
        }
        // Two-member swarms debounce SwarmStatus in a separate Tokio task.
        // Observe the actual startup publication before installing a successor,
        // so its pending broadcast cannot be mistaken for terminal publication
        // or left dirty across this fixture's private runtime teardown.
        let startup_status = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(event) = member_rx.recv().await {
                if matches!(event, ServerEvent::SwarmStatus { members }
                    if members.iter().any(|member| member.session_id == worker && member.status == "running"))
                {
                    return true;
                }
            }
            false
        }).await;
        if !matches!(startup_status, Ok(true)) {
            handle.abort();
            let _ = handle.await;
            panic!("actor startup status was not published: {case}");
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(plans.read().await[swarm_id].items[0].status, "running");
        let snapshot = crate::storage::durable_state_dir()
            .join("swarm")
            .join(format!("{swarm_id}.json"));
        assert!(snapshot.is_file(), "startup must have saved before the barrier");

        // Exercise the real graph handlers, including their owner/progress
        // asymmetry, rather than substituting status-only fixtures.
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        if case.starts_with("expand_") || case.starts_with("join_") {
            handle_comm_expand_node(
                101, worker.to_string(), "task".to_string(),
                vec![node_spec("task.child", "explore", &[])],
                &client_tx, &members, &swarms, &plans, &coordinators,
                &history, &counter, &event_tx,
            ).await;
            assert!(matches!(client_rx.try_recv().unwrap(), ServerEvent::Done { id: 101 }));
            let locked = plans.read().await;
            let plan = &locked[swarm_id];
            let task = plan.items.iter().find(|item| item.id == "task").unwrap();
            assert_eq!(task.status, "queued");
            assert!(task.assigned_to.is_none());
            assert!(plan.node_meta["task"].expanded);
            assert!(!plan.node_meta["task"].is_gate);
            assert_eq!(plan.task_progress["task"].assigned_session_id.as_deref(), Some(worker));
        }
        if case == "complete_node" || case.starts_with("join_") {
            let mut complete_ids = if case == "complete_node" {
                vec!["task".to_string()]
            } else {
                vec!["task.child".to_string()]
            };
            if case.starts_with("join_deep") {
                let locked = plans.read().await;
                complete_ids.extend(locked[swarm_id].node_meta.iter()
                    .filter(|(_, meta)| meta.is_gate && meta.parent.as_deref() == Some("task"))
                    .map(|(id, _)| id.clone()));
                assert_eq!(complete_ids.len(), 2, "real deep expansion must insert its gate");
            }
            for id in complete_ids {
                handle_comm_complete_node(
                    102, worker.to_string(), id.clone(),
                    serde_json::json!({
                        "findings": "task.child checked with deterministic local evidence",
                        "confidence": "high",
                        "what_i_did_not_check": ["external services"],
                    }).to_string(),
                    &client_tx, &members, &swarms, &plans, &coordinators,
                    &history, &counter, &event_tx,
                ).await;
                assert!(matches!(client_rx.try_recv().unwrap(), ServerEvent::Done { id: 102 }),
                    "real completion failed for {id}");
                let locked = plans.read().await;
                assert_eq!(locked[swarm_id].items.iter().find(|item| item.id == id).unwrap().status, "completed");
                assert!(locked[swarm_id].node_meta[&id].artifact_json.is_some());
            }
            if case.starts_with("join_") {
                // The actual DAG scheduler admits synthesis only after its
                // child and (in deep mode) gate have really completed.
                let mut locked = plans.write().await;
                let plan = locked.get_mut(swarm_id).unwrap();
                let mut graph = crate::plan::bridge::to_task_graph(plan);
                assert!(crate::plan::dag::dispatch(&mut graph, "task", worker));
                crate::plan::bridge::apply_task_graph(plan, &graph);
                plan.version += 1;
                assert_eq!(plan.task_progress["task"].assigned_session_id.as_deref(), Some(worker));
            } else {
                // At this base complete_node preserves its actor owner. Do not
                // invent owner clearing here: absent-owner cases are separate.
                assert_eq!(plans.read().await[swarm_id].task_progress["task"].assigned_session_id.as_deref(), Some(worker));
            }
        }

        {
            let mut locked = plans.write().await;
            if case == "missing_plan" {
                locked.remove(swarm_id);
            } else {
                let plan = locked.get_mut(swarm_id).unwrap();
                let task = plan.items.iter_mut().find(|item| item.id == "task").unwrap();
                match case {
                    "missing_task" => plan.items.retain(|item| item.id != "task"),
                    "missing_progress" => { plan.task_progress.remove("task"); }
                    "item_owner" => task.assigned_to = Some("successor".to_string()),
                    "progress_owner" => plan.task_progress.get_mut("task").unwrap().assigned_session_id = Some("successor".to_string()),
                    "both_owners" => {
                        task.assigned_to = Some("successor".to_string());
                        task.content = "successor's independent brief".to_string();
                        let progress = plan.task_progress.get_mut("task").unwrap();
                        progress.assigned_session_id = Some("successor".to_string());
                        progress.assignment_summary = Some("successor assignment".to_string());
                        progress.last_detail = Some("successor detail".to_string());
                        progress.checkpoint_summary = Some("successor checkpoint".to_string());
                        progress.heartbeat_count = Some(41);
                        progress.checkpoint_count = Some(29);
                        plan.version += 10;
                    }
                    "absent_item_owner" => task.assigned_to = None,
                    "absent_progress_owner" => plan.task_progress.get_mut("task").unwrap().assigned_session_id = None,
                    "running_stale" => {
                        task.status = case.to_string();
                        plan.task_progress.get_mut("task").unwrap().stale_since_unix_ms = Some(222);
                    }
                    "queued" | "done" | "completed" | "failed" | "cancelled" | "pending" | "blocked" | "unknown" => task.status = case.to_string(),
                    _ => {}
                }
            }
        }

        // Install writer faults only after the provider was reached. These are
        // terminal save controls, not zero-provider startup rejection tests.
        let parked = temp.path().join("parked-swarm");
        if case == "save_error" {
            std::fs::rename(snapshot.parent().unwrap(), &parked).unwrap();
            std::fs::write(snapshot.parent().unwrap(), b"not a directory").unwrap();
        } else if case == "save_stale" {
            let mut newer = plans.read().await[swarm_id].clone();
            newer.version += 100;
            crate::server::swarm_persistence::persist_swarm_state(
                swarm_id, Some(&newer), Some("coord"), &[],
            );
            let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&snapshot).unwrap()).unwrap();
            assert_eq!(saved["plan"]["version"], newer.version);
        }
        let before_snapshot = std::fs::read(&snapshot).ok();
        let before_backup = std::fs::read(snapshot.with_extension("bak")).ok();
        let before_parked = std::fs::read(parked.join(format!("{swarm_id}.json"))).ok();
        let before_plan = plans.read().await.get(swarm_id).cloned();
        while member_rx.try_recv().is_ok() {}
        while event_rx.try_recv().is_ok() {}
        history.write().await.clear();
        let before_counter = counter.load(std::sync::atomic::Ordering::SeqCst);
        release.notify_one();
        match tokio::time::timeout(Duration::from_secs(10), &mut handle).await {
            Ok(result) => result.expect("terminal runner panicked"),
            Err(_) => {
                handle.abort();
                let _ = handle.await;
                panic!("terminal runner timed out: {case}, fail={fail}");
            }
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);

        // A rejected PLAN transition must not return from the runner before its
        // independent actor completion/failure and coordinator report happen.
        let expected_member_status = if fail { "failed" } else { "completed" };
        {
            let locked = members.read().await;
            assert_eq!(locked[worker].status, expected_member_status);
            if fail {
                assert!(locked[worker].detail.as_deref().unwrap().contains("inert terminal failure"));
            } else {
                assert_eq!(locked[worker].latest_completion_report.as_deref(), Some("inert actor report"));
            }
        }
        let events = history.read().await;
        assert_eq!(events.len(), 1, "only the actor's terminal status is a history event");
        assert!(matches!(&events[0].event, SwarmEventType::StatusChange { old_status, new_status }
            if old_status == "running" && new_status == expected_member_status));
        assert_eq!(events[0].session_id, worker);
        drop(events);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), before_counter + 1);
        assert!(matches!(event_rx.try_recv().unwrap().event, SwarmEventType::StatusChange { new_status, .. }
            if new_status == expected_member_status));
        assert!(matches!(event_rx.try_recv(), Err(broadcast::error::TryRecvError::Empty)));
        let mut plan_events = Vec::new();
        let mut actor_notification = false;
        let mut member_status_event = false;
        // Joining the runner guarantees its status mutation/history/report,
        // not delivery by broadcast_swarm_status's debounced task. Await real
        // wire events, allowing paused Tokio time to advance to that task.
        // Collect every preceding task-plan event too, rather than discarding
        // the very terminal effects the rejection cases must prohibit.
        let terminal_publication = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(event) = member_rx.recv().await {
                match event {
                    ServerEvent::SwarmPlan { reason, version, summary, .. } => plan_events.push((reason, version, summary)),
                    ServerEvent::SwarmStatus { members } => member_status_event |= members.iter()
                        .any(|member| member.session_id == worker && member.status == expected_member_status),
                    ServerEvent::Notification { from_session, message, .. } => {
                        actor_notification |= from_session == worker
                            && message.contains(if fail { "failed" } else { "inert actor report" });
                    }
                    _ => {}
                }
                if member_status_event && actor_notification {
                    return;
                }
            }
        }).await;
        assert!(terminal_publication.is_ok(),
            "actor publication timed out: {case}, fail={fail}, status={member_status_event}, report={actor_notification}");
        assert!(member_status_event, "actor status must still be published");
        assert!(actor_notification, "actor report/failure must still reach its coordinator");

        let eligible = matches!(case, "owned" | "running_stale" | "deep_first" | "deep_repeated");
        let locked = plans.read().await;
        if !eligible {
            // Debug includes every VersionedPlan field, including every task's
            // progress counters and artifacts, without lossy projection.
            assert_eq!(format!("{:?}", locked.get(swarm_id)), format!("{:?}", before_plan.as_ref()),
                "rejected terminal plan changed: {case}, fail={fail}");
            assert!(plan_events.is_empty(), "rejected terminal task broadcast: {plan_events:?}");
            assert_eq!(std::fs::read(&snapshot).ok(), before_snapshot, "rejected terminal persisted");
            assert_eq!(std::fs::read(snapshot.with_extension("bak")).ok(), before_backup);
            if case == "save_error" {
                assert_eq!(std::fs::read(snapshot.parent().unwrap()).unwrap(), b"not a directory");
                assert_eq!(std::fs::read(parked.join(format!("{swarm_id}.json"))).ok(), before_parked);
            }
        } else {
            let plan = &locked[swarm_id];
            let before = before_plan.as_ref().unwrap();
            let task = plan.items.iter().find(|item| item.id == "task").unwrap();
            let progress = &plan.task_progress["task"];
            let old_progress = &before.task_progress["task"];
            let (status, reason) = if fail {
                ("failed", "task_failed")
            } else if case == "deep_first" {
                ("queued", "task_requeued_no_artifact")
            } else if case == "deep_repeated" {
                ("failed", "task_failed_no_artifact")
            } else {
                ("done", "task_completed")
            };
            assert_eq!(task.status, status);
            assert_eq!(plan.version, before.version + 1);
            assert_eq!(progress.checkpoint_count, Some(old_progress.checkpoint_count.unwrap() + 1));
            assert_eq!(progress.heartbeat_count, old_progress.heartbeat_count);
            assert_eq!(progress.dead_assignee_reclaims, old_progress.dead_assignee_reclaims);
            assert_eq!(progress.assigned_at_unix_ms, old_progress.assigned_at_unix_ms);
            assert_eq!(progress.started_at_unix_ms, old_progress.started_at_unix_ms);
            assert!(progress.last_checkpoint_unix_ms.is_some());
            assert!(progress.stale_since_unix_ms.is_none());
            if !fail && case == "deep_first" {
                assert!(task.assigned_to.is_none());
                assert!(progress.assigned_session_id.is_none());
                assert_eq!(progress.no_artifact_requeues, Some(1));
                assert!(progress.completed_at_unix_ms.is_none());
            } else {
                assert_eq!(task.assigned_to.as_deref(), Some(worker));
                assert_eq!(progress.assigned_session_id.as_deref(), Some(worker));
                assert_eq!(progress.no_artifact_requeues, old_progress.no_artifact_requeues);
                assert!(progress.completed_at_unix_ms.is_some());
            }
            assert_eq!(plan.node_meta, before.node_meta);
            assert_eq!(plan.items[1..], before.items[1..]);
            assert_eq!(plan_events.len(), 1);
            assert_eq!(plan_events[0].0.as_deref(), Some(reason));
            assert_eq!(plan_events[0].1, plan.version);
            assert_eq!(
                plan_events[0].2.as_ref().unwrap().newly_ready_ids,
                if status == "done" {
                    vec!["dependent".to_string()]
                } else if !fail && case == "deep_first" {
                    vec!["task".to_string()]
                } else {
                    Vec::new()
                },
                "task effects must describe only the accepted terminal transition",
            );
            let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&snapshot).unwrap()).unwrap();
            assert_eq!(saved["plan"]["version"], plan.version);
            assert_eq!(saved["plan"]["items"], serde_json::to_value(&plan.items).unwrap());
            assert_eq!(saved["plan"]["task_progress"], serde_json::to_value(&plan.task_progress).unwrap());
            assert_eq!(saved["plan"]["node_meta"], serde_json::to_value(&plan.node_meta).unwrap());
        }
    });
}

macro_rules! lifecycle_current_owner_terminal_cases {
    ($($ok:ident, $err:ident => $case:literal),+ $(,)?) => {$(
        #[test]
        fn $ok() { check_current_owner_terminal($case, false); }
        #[test]
        fn $err() { check_current_owner_terminal($case, true); }
    )+};
}

lifecycle_current_owner_terminal_cases! {
    lifecycle_current_owner_terminal_missing_plan_ok, lifecycle_current_owner_terminal_missing_plan_err => "missing_plan",
    lifecycle_current_owner_terminal_missing_task_ok, lifecycle_current_owner_terminal_missing_task_err => "missing_task",
    lifecycle_current_owner_terminal_missing_progress_ok, lifecycle_current_owner_terminal_missing_progress_err => "missing_progress",
    lifecycle_current_owner_terminal_item_owner_ok, lifecycle_current_owner_terminal_item_owner_err => "item_owner",
    lifecycle_current_owner_terminal_progress_owner_ok, lifecycle_current_owner_terminal_progress_owner_err => "progress_owner",
    lifecycle_current_owner_terminal_both_owners_ok, lifecycle_current_owner_terminal_both_owners_err => "both_owners",
    lifecycle_current_owner_terminal_absent_item_owner_ok, lifecycle_current_owner_terminal_absent_item_owner_err => "absent_item_owner",
    lifecycle_current_owner_terminal_absent_progress_owner_ok, lifecycle_current_owner_terminal_absent_progress_owner_err => "absent_progress_owner",
    lifecycle_current_owner_terminal_queued_ok, lifecycle_current_owner_terminal_queued_err => "queued",
    lifecycle_current_owner_terminal_done_ok, lifecycle_current_owner_terminal_done_err => "done",
    lifecycle_current_owner_terminal_completed_ok, lifecycle_current_owner_terminal_completed_err => "completed",
    lifecycle_current_owner_terminal_failed_ok, lifecycle_current_owner_terminal_failed_err => "failed",
    lifecycle_current_owner_terminal_cancelled_ok, lifecycle_current_owner_terminal_cancelled_err => "cancelled",
    lifecycle_current_owner_terminal_pending_ok, lifecycle_current_owner_terminal_pending_err => "pending",
    lifecycle_current_owner_terminal_blocked_ok, lifecycle_current_owner_terminal_blocked_err => "blocked",
    lifecycle_current_owner_terminal_unknown_ok, lifecycle_current_owner_terminal_unknown_err => "unknown",
    lifecycle_current_owner_terminal_complete_node_ok, lifecycle_current_owner_terminal_complete_node_err => "complete_node",
    lifecycle_current_owner_terminal_expand_light_ok, lifecycle_current_owner_terminal_expand_light_err => "expand_light",
    lifecycle_current_owner_terminal_expand_deep_ok, lifecycle_current_owner_terminal_expand_deep_err => "expand_deep",
    lifecycle_current_owner_terminal_join_light_ok, lifecycle_current_owner_terminal_join_light_err => "join_light",
    lifecycle_current_owner_terminal_join_deep_first_ok, lifecycle_current_owner_terminal_join_deep_first_err => "join_deep_first",
    lifecycle_current_owner_terminal_join_deep_repeated_ok, lifecycle_current_owner_terminal_join_deep_repeated_err => "join_deep_repeated",
    lifecycle_current_owner_terminal_save_error_ok, lifecycle_current_owner_terminal_save_error_err => "save_error",
    lifecycle_current_owner_terminal_save_stale_ok, lifecycle_current_owner_terminal_save_stale_err => "save_stale",
    lifecycle_current_owner_terminal_owned_ok, lifecycle_current_owner_terminal_owned_err => "owned",
    lifecycle_current_owner_terminal_running_stale_ok, lifecycle_current_owner_terminal_running_stale_err => "running_stale",
    lifecycle_current_owner_terminal_deep_first_ok, lifecycle_current_owner_terminal_deep_first_err => "deep_first",
    lifecycle_current_owner_terminal_deep_repeated_ok, lifecycle_current_owner_terminal_deep_repeated_err => "deep_repeated",
}

include!("comm_control_tests/assign_task.rs");
include!("comm_control_tests/assign_blocked.rs");
include!("comm_control_tests/assign_double.rs");
include!("comm_control_tests/assign_ready_agent.rs");
include!("comm_control_tests/assign_less_loaded.rs");
include!("comm_control_tests/assign_busy_skip.rs");
include!("comm_control_tests/task_control.rs");
include!("comm_control_tests/assign_next_dependency.rs");
include!("comm_control_tests/assign_next_metadata.rs");
include!("comm_control_tests/await_late_joiners.rs");
include!("comm_control_tests/await_disconnect.rs");
include!("comm_control_tests/await_any.rs");
include!("comm_control_tests/await_reload_deadline.rs");
include!("comm_control_tests/await_reload_final.rs");
include!("comm_control_tests/await_lagged.rs");
include!("comm_control_tests/await_resume_expired.rs");
include!("comm_control_tests/await_background_expired.rs");
include!("comm_control_tests/await_upgrade_background.rs");
include!("comm_control_tests/dag_e2e.rs");
include!("comm_control_tests/auto_worker_filter.rs");
include!("comm_control_tests/client_attached_dispatch.rs");
