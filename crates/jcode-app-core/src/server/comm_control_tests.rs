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

fn check_detached_task_start_current_owner(case: &str) {
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
        let mut target = member(worker, swarm_id, "ready");
        target.event_tx = member_tx;
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
        item.assigned_to = Some(worker.to_string());
        let mut plan = VersionedPlan::new();
        plan.items.push(item);
        plan.version = 7;
        plan.participants.insert(worker.to_string());
        plan.task_progress
            .entry("task".to_string())
            .or_default()
            .assigned_session_id = Some(worker.to_string());
        let plans = Arc::new(RwLock::new(HashMap::from([(swarm_id.to_string(), plan)])));
        let history = Arc::new(RwLock::new(VecDeque::new()));
        let counter = Arc::new(AtomicU64::new(7));
        let (event_tx, mut event_rx) = broadcast::channel(64);
        let snapshot = crate::storage::durable_state_dir()
            .join("swarm")
            .join(format!("{swarm_id}.json"));
        assert!(!snapshot.exists());

        if case.starts_with("requeue_") {
            {
                let mut locked = plans.write().await;
                match case {
                    "requeue_stale" => {
                        locked.get_mut(swarm_id).unwrap().items[0].status =
                            "running_stale".to_string();
                    }
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
                &plans,
            )
            .await;
            assert_eq!(requeued.is_some(), case == "requeue_stale");
            if requeued.is_some() {
                assert_eq!(plans.read().await[swarm_id].items[0].status, "queued");
            }
        }

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
                    "queued" | "requeue_stale" => {}
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
        let calls = calls.load(std::sync::atomic::Ordering::SeqCst);
        if matches!(case, "queued" | "requeue_stale") {
            assert!(
                calls > 0,
                "valid queued owner must reach the inert provider"
            );
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
            assert!(!snapshot.exists(), "rejection persisted a swarm snapshot");
        }
    });
}

macro_rules! detached_task_start_current_owner_cases {
    ($($name:ident => $case:literal),+ $(,)?) => {$(
        #[test]
        fn $name() {
            check_detached_task_start_current_owner($case);
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
