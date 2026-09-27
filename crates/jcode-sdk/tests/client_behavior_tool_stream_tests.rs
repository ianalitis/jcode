#[test]
fn tool_names_arrive_before_any_arguments_and_interleaved_inputs_keep_call_ids() {
    let (release, wait) = channel();
    let client = fake_harness(move |frame, writer| {
        if let ApiRequest::Ping = frame.request {
            reply(frame, ApiEvent::Pong, writer);
            for id in ["a", "b"] {
                push(
                    ApiEvent::ToolStart {
                        session_id: "mine".into(),
                        call_id: id.into(),
                        name: "bash".into(),
                    },
                    writer,
                );
            }
            // The SDK must deliver both names with the connection open and no
            // argument frame available. No transport EOF can mask buffering.
            wait.recv_timeout(Duration::from_secs(5))
                .expect("client observed names");
            for (id, delta) in [("b", "{\"command\":"), ("a", "{}"), ("b", "\"pwd\"}")] {
                push(
                    ApiEvent::ToolInputDelta {
                        session_id: "mine".into(),
                        call_id: id.into(),
                        delta: delta.into(),
                    },
                    writer,
                );
            }
        }
    });
    let events = client.events(Some("mine"));
    client.ping().unwrap();
    for id in ["a", "b"] {
        assert!(
            matches!(events.next_timeout(Duration::from_secs(5)), Some(ApiEvent::ToolStart { call_id, name, .. }) if call_id == id && name == "bash")
        );
    }
    release.send(()).unwrap();
    for (id, fragment) in [("b", "{\"command\":"), ("a", "{}"), ("b", "\"pwd\"}")] {
        assert!(
            matches!(events.next_timeout(Duration::from_secs(5)), Some(ApiEvent::ToolInputDelta { call_id, delta, .. }) if call_id == id && delta == fragment)
        );
    }
}
