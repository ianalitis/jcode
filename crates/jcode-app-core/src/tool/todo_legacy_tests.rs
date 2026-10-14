#[test]
fn garbage_string_still_errors() {
    assert!(parse(json!({"todos": "not json at all"})).is_err());
}

/// Sessions and model calls written before the rename carry
/// `hill_climbability`. Those must keep loading, or resuming an old session
/// silently drops its goal assessments and re-raises resolved gate points.
#[test]
fn pre_rename_hill_climbability_keys_still_load() {
    let goal: crate::todo::TodoGoal = serde_json::from_value(json!({
        "group": "optimize grep",
        "hill_climbability": 91,
        "hill_climbability_history": [70, 91],
        "feedback_loop": "cargo bench grep"
    }))
    .expect("the pre-rename key must still deserialize");
    assert_eq!(
        goal.closed_feedback_loop,
        Some(crate::todo::FeedbackLoopState::Strong)
    );
    assert_eq!(
        goal.closed_feedback_loop_history,
        vec![
            crate::todo::FeedbackLoopState::Usable,
            crate::todo::FeedbackLoopState::Strong
        ]
    );

    let goals = parse(json!({
        "goals": [{"group": "optimize grep", "hill_climbability": "88", "feedback_loop": "bench"}]
    }))
    .expect("a pre-rename tool call must still parse")
    .goals
    .expect("goals should be present");
    assert_eq!(
        goals[0].closed_feedback_loop,
        Some(crate::todo::FeedbackLoopState::Strong)
    );
}
