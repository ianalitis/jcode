//! Route-table admission: turn a node's declared route axes into a frozen
//! attempt.
//!
//! This is the "before" authority point of `docs/HARNESS_LOOP_ARCHITECTURE.md`.
//! It is pure: it reads a validated [`RouteTable`] and a [`TaskNode`], resolves
//! the promoted route for the node's task class and data class, and freezes an
//! [`AttemptRecord`]. Nothing here mutates the graph, spawns an executor or
//! touches the network.

use super::TaskNode;
use chrono::{DateTime, Utc};
use jcode_attempt_types::{
    AttemptRecord, FreezeError, FrozenAttempt, LocalBudget, RouteTable, RouteTableError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    /// The node never declared a task class, so there is no route to admit.
    MissingTaskClass { node: String },
    /// The route table refused the task class or its data class.
    Route(RouteTableError),
    /// The composed record was not a valid frozen envelope.
    Freeze(FreezeError),
}

impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdmissionError::MissingTaskClass { node } => {
                write!(
                    f,
                    "node `{node}` has no task class, so it cannot be admitted"
                )
            }
            AdmissionError::Route(e) => write!(f, "route admission refused: {e}"),
            AdmissionError::Freeze(e) => write!(f, "frozen attempt rejected: {e}"),
        }
    }
}

impl std::error::Error for AdmissionError {}

/// Caller-owned values that are not part of the route decision.
#[derive(Debug, Clone)]
pub struct AdmissionParams<'a> {
    /// Unique attempt id for this execution.
    pub attempt_id: &'a str,
    /// Digest of the exact packet the executor will receive. The caller owns
    /// this because it knows the assembled input, not just the node content.
    pub prompt_hash: &'a str,
    pub deadline_secs: u64,
    pub budget: LocalBudget,
    pub policy_version: &'a str,
}

/// Resolve the node's promoted route and freeze its attempt envelope.
///
/// A node without a declared task class is refused rather than defaulted. A
/// node without a declared data class is treated as the most restrictive class
/// (Private), so it can only be admitted to a route vetted for Private.
pub fn admit_node(
    node: &TaskNode,
    table: &RouteTable,
    params: AdmissionParams<'_>,
    frozen_at: DateTime<Utc>,
) -> Result<FrozenAttempt, AdmissionError> {
    let task_class =
        node.task_class
            .as_deref()
            .ok_or_else(|| AdmissionError::MissingTaskClass {
                node: node.id.clone(),
            })?;
    let data_class = node.data_class.unwrap_or_default();
    let entry = table
        .resolve(task_class, data_class)
        .map_err(AdmissionError::Route)?;
    AttemptRecord {
        task_id: node.id.clone(),
        attempt_id: params.attempt_id.to_string(),
        node_id: node.id.clone(),
        provider: entry.provider.clone(),
        model_exact: entry.model_exact.clone(),
        endpoint: entry.endpoint.clone(),
        route_class: entry.route_class,
        effort: entry.effort,
        // Admission never grants tools; a tool allowlist is a separate,
        // narrower decision made by the spawn envelope.
        tool_allowlist: Vec::new(),
        data_class,
        deadline_secs: params.deadline_secs,
        budget: params.budget,
        prompt_hash: params.prompt_hash.to_string(),
        policy_version: params.policy_version.to_string(),
    }
    .freeze(frozen_at)
    .map_err(AdmissionError::Freeze)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{NodeKind, NodeOrigin, NodeStatus};
    use chrono::TimeZone;
    use jcode_attempt_types::{DataClass, Effort, RouteClass, RouteEntry};

    fn node(task_class: Option<&str>, data_class: Option<DataClass>) -> TaskNode {
        TaskNode {
            id: "node0".into(),
            content: "classify the intake brief".into(),
            kind: NodeKind::Explore,
            status: NodeStatus::Queued,
            owner: None,
            parent: None,
            depends_on: Vec::new(),
            expanded: false,
            is_gate: false,
            planner: None,
            priority: 0,
            output: None,
            origin: Some(NodeOrigin::Seed),
            attempt_id: None,
            task_class: task_class.map(str::to_string),
            data_class,
        }
    }

    fn table(admitted: DataClass, route: RouteClass) -> RouteTable {
        let mut t = RouteTable::default();
        t.promote(
            "intake.classify",
            RouteEntry {
                provider: "openrouter".into(),
                model_exact: "deepseek/deepseek-v4-flash-0731".into(),
                endpoint: "https://openrouter.ai/api/v1".into(),
                route_class: route,
                effort: Effort::Medium,
                admitted_data_class: admitted,
                promoted_at: Utc.with_ymd_and_hms(2026, 9, 18, 20, 0, 0).unwrap(),
                evidence_path: "/scratch/p2/RECEIPT.md".into(),
                policy_version: "2026-09-18".into(),
            },
        );
        t
    }

    fn params() -> AdmissionParams<'static> {
        AdmissionParams {
            attempt_id: "node0/a-1",
            prompt_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            deadline_secs: 60,
            budget: LocalBudget::default(),
            policy_version: "2026-09-18",
        }
    }

    fn frozen_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 18, 20, 0, 0).unwrap()
    }

    #[test]
    fn admits_a_declared_class_to_its_promoted_route() {
        let t = table(DataClass::Synthetic, RouteClass::MeteredRemote);
        let n = node(Some("intake.classify"), Some(DataClass::Synthetic));
        let frozen = admit_node(&n, &t, params(), frozen_at()).unwrap();
        assert_eq!(frozen.attempt_id(), "node0/a-1");
        let r = frozen.record();
        assert_eq!(r.model_exact, "deepseek/deepseek-v4-flash-0731");
        assert_eq!(r.route_class, RouteClass::MeteredRemote);
        assert_eq!(r.data_class, DataClass::Synthetic);
        assert!(r.tool_allowlist.is_empty(), "admission grants no tools");
    }

    #[test]
    fn refuses_a_node_without_a_task_class() {
        let t = table(DataClass::Synthetic, RouteClass::MeteredRemote);
        let n = node(None, Some(DataClass::Synthetic));
        assert_eq!(
            admit_node(&n, &t, params(), frozen_at()),
            Err(AdmissionError::MissingTaskClass {
                node: "node0".into()
            })
        );
    }

    #[test]
    fn undeclared_data_class_is_treated_as_private_and_refused_by_a_public_route() {
        let t = table(DataClass::Public, RouteClass::IncludedSubscription);
        let n = node(Some("intake.classify"), None);
        let err = admit_node(&n, &t, params(), frozen_at()).unwrap_err();
        assert!(
            matches!(
                err,
                AdmissionError::Route(RouteTableError::OutsidePromotionScope { .. })
            ),
            "{err}"
        );
    }

    #[test]
    fn unknown_task_class_refuses() {
        let t = table(DataClass::Synthetic, RouteClass::MeteredRemote);
        let n = node(Some("factory.theme-pick"), Some(DataClass::Synthetic));
        let err = admit_node(&n, &t, params(), frozen_at()).unwrap_err();
        assert!(
            matches!(
                err,
                AdmissionError::Route(RouteTableError::UnknownTaskClass(_))
            ),
            "{err}"
        );
    }
}
