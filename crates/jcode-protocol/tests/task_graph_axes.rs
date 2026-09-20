use jcode_protocol::TaskGraphNodeSpec;
use serde_json::json;

#[test]
fn declared_axes_round_trip_for_every_data_class() {
    for data_class in ["public", "synthetic", "private", "secret"] {
        let input = json!({
            "id": "intake",
            "content": "classify intake",
            "task_class": "factory.intake.classify",
            "data_class": data_class
        });
        let spec: TaskGraphNodeSpec = serde_json::from_value(input).unwrap();
        let output = serde_json::to_value(spec).unwrap();
        assert_eq!(output["task_class"], "factory.intake.classify");
        assert_eq!(output["data_class"], data_class);
    }
}

#[test]
fn legacy_nodes_do_not_gain_declared_axes() {
    let input = json!({"id": "legacy", "content": "legacy task"});
    let spec: TaskGraphNodeSpec = serde_json::from_value(input).unwrap();
    let output = serde_json::to_value(spec).unwrap();
    assert!(output.get("task_class").is_none());
    assert!(output.get("data_class").is_none());
}

#[test]
fn invalid_data_classes_are_rejected_at_the_wire_boundary() {
    for data_class in [json!("PUBLIC"), json!("unknown"), json!(42), json!({})] {
        let input = json!({"id": "bad", "content": "task", "data_class": data_class});
        assert!(serde_json::from_value::<TaskGraphNodeSpec>(input).is_err());
    }
}
