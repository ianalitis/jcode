use super::*;

#[derive(Clone, Default)]
struct SdkCallbackProvider {
    calls: Arc<std::sync::atomic::AtomicUsize>,
}
#[async_trait]
impl Provider for SdkCallbackProvider {
    async fn complete(
        &self,
        _: &[Message],
        tools: &[ToolDefinition],
        _: &str,
        _: Option<&str>,
    ) -> Result<EventStream> {
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read");
        assert_eq!(tools[0].description, "SDK read callback");
        let events = if self.calls.fetch_add(1, Ordering::SeqCst).is_multiple_of(2) {
            vec![
                StreamEvent::ToolUseStart {
                    id: "model-call".into(),
                    name: "read".into(),
                },
                StreamEvent::ToolInputDelta("{}".into()),
                StreamEvent::ToolUseEnd,
                StreamEvent::MessageEnd {
                    stop_reason: Some("tool_use".into()),
                },
            ]
        } else {
            vec![
                StreamEvent::TextDelta("callback completed".into()),
                StreamEvent::MessageEnd {
                    stop_reason: Some("end_turn".into()),
                },
            ]
        };
        Ok(Box::pin(stream::iter(events.into_iter().map(Ok))))
    }
    fn name(&self) -> &str {
        "sdk-callback-test"
    }
    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

#[tokio::test]
async fn sdk_socket_configure_list_callback_and_busy_rejection() {
    sdk_socket_workflow(false, false, false).await;
}

#[tokio::test]
async fn user_response_socket_registration_follows_explicit_capability() {
    sdk_socket_workflow(true, true, false).await;
}

#[tokio::test]
async fn user_response_socket_observer_first_rebind_and_disconnect() {
    sdk_socket_workflow(true, false, false).await;
}

#[tokio::test]
async fn user_response_socket_observer_clear_keeps_desktop_attached() {
    sdk_socket_workflow(true, false, true).await;
}

async fn sdk_socket_workflow(response_only: bool, desktop_first: bool, clear_shared: bool) {
    let _lock = crate::storage::lock_test_env();
    let home = IsolatedReloadRecoveryEnv::new();
    let provider_template: Arc<dyn Provider> = Arc::new(SdkCallbackProvider::default());

    let sessions: SessionAgents = Arc::new(RwLock::new(HashMap::new()));
    let global_session_id = Arc::new(RwLock::new(String::new()));
    let client_count = Arc::new(RwLock::new(0usize));
    let client_connections = Arc::new(RwLock::new(HashMap::new()));
    let swarm_members = Arc::new(RwLock::new(HashMap::new()));
    let swarms_by_id = Arc::new(RwLock::new(HashMap::new()));
    let shared_context = Arc::new(RwLock::new(HashMap::new()));
    let swarm_plans = Arc::new(RwLock::new(HashMap::new()));
    let swarm_coordinators = Arc::new(RwLock::new(HashMap::new()));
    let file_touch = FileTouchService::new();
    let channel_subscriptions = Arc::new(RwLock::new(HashMap::new()));
    let channel_subscriptions_by_session = Arc::new(RwLock::new(HashMap::new()));
    let client_debug_state = Arc::new(RwLock::new(ClientDebugState::default()));
    let (_debug_response_tx, _) = broadcast::channel(8);
    let event_history = Arc::new(RwLock::new(std::collections::VecDeque::new()));
    let event_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let (swarm_event_tx, _) = broadcast::channel(8);
    let (_global_event_tx, _) = broadcast::channel(8);
    let global_is_processing = Arc::new(RwLock::new(false));
    let shutdown_signals = Arc::new(RwLock::new(HashMap::new()));
    let soft_interrupt_queues: SessionInterruptQueues = Arc::new(RwLock::new(HashMap::new()));
    let mcp_pool = Arc::new(crate::mcp::SharedMcpPool::from_default_config());

    let connect = || {
        let (server_stream, client_stream) = crate::transport::Stream::pair().expect("socket pair");
        let server_task = tokio::spawn(handle_client(
            server_stream,
            Arc::clone(&sessions),
            _global_event_tx.clone(),
            provider_template.clone(),
            global_is_processing.clone(),
            global_session_id.clone(),
            client_count.clone(),
            Arc::clone(&client_connections),
            swarm_members.clone(),
            swarms_by_id.clone(),
            shared_context.clone(),
            swarm_plans.clone(),
            swarm_coordinators.clone(),
            file_touch.clone(),
            channel_subscriptions.clone(),
            channel_subscriptions_by_session.clone(),
            client_debug_state.clone(),
            _debug_response_tx.clone(),
            event_history.clone(),
            event_counter.clone(),
            swarm_event_tx.clone(),
            "jcode-test".to_string(),
            "🧪".to_string(),
            mcp_pool.clone(),
            shutdown_signals.clone(),
            soft_interrupt_queues.clone(),
            AwaitMembersRuntime::default(),
            SwarmMutationRuntime::default(),
        ));
        (server_task, client_stream)
    };
    let (mut server_task, client_stream) = connect();

    let (client_reader, mut client_writer) = client_stream.into_split();
    let mut client_reader = BufReader::new(client_reader);

    async fn send(writer: &mut crate::transport::WriteHalf, value: serde_json::Value) {
        writer
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
    }
    async fn until(
        reader: &mut BufReader<crate::transport::ReadHalf>,
        predicate: impl Fn(&ServerEvent) -> bool,
    ) -> ServerEvent {
        let mut last_event = String::new();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).await.unwrap() > 0);
                let event: ServerEvent = serde_json::from_str(&line).unwrap();
                last_event = format!("{event:?}");
                if predicate(&event) {
                    return event;
                }
            }
        })
        .await
        .unwrap_or_else(|error| panic!("daemon event timeout: {error}; last event: {last_event}"))
    }
    async fn done(reader: &mut BufReader<crate::transport::ReadHalf>, id: u64) {
        until(reader, |e| {
            assert!(
                !matches!(e, ServerEvent::Error { id: failed, .. } if *failed == id),
                "{e:?}"
            );
            matches!(e, ServerEvent::Done { id: completed } if *completed == id)
        })
        .await;
    }
    async fn response_tool(
        reader: &mut BufReader<crate::transport::ReadHalf>,
        writer: &mut crate::transport::WriteHalf,
        expected: bool,
    ) {
        send(writer, serde_json::json!({"type":"list_tools","id":900})).await;
        let ServerEvent::Tools { tools, .. } =
            until(reader, |e| matches!(e, ServerEvent::Tools { id: 900, .. })).await
        else {
            panic!("expected tool list")
        };
        assert_eq!(
            tools.iter().any(|tool| tool.name == "user_response"),
            expected,
            "user_response must reflect all currently attached clients"
        );
    }
    use serde_json::json;
    send(
        &mut client_writer,
        json!({"type":"subscribe","id":1,"working_dir":home._home.path()}),
    )
    .await;
    until(&mut client_reader, |e| {
        matches!(e, ServerEvent::Done { id: 1 })
    })
    .await;
    if response_only {
        // Exercise actual Desktop/TUI snapshots, including reversible disabling.
        for (index, capability) in [false, true, false].into_iter().enumerate() {
            let id = 100 + index as u64;
            send(
                &mut client_writer,
                json!({"type":"subscribe","id":id,
                "working_dir":home._home.path(),"supports_pdf_panels":true,
                "supports_user_response":capability}),
            )
            .await;
            done(&mut client_reader, id).await;
            response_tool(&mut client_reader, &mut client_writer, capability).await;
        }
        send(
            &mut client_writer,
            json!({"type":"subscribe","id":110,
            "working_dir":home._home.path(),"supports_user_response":desktop_first}),
        )
        .await;
        done(&mut client_reader, 110).await;
        let session_id = sessions.read().await.keys().next().unwrap().clone();
        let (mut observer_task, observer_stream) = connect();
        let (observer_reader, mut observer_writer) = observer_stream.into_split();
        let mut observer_reader = BufReader::new(observer_reader);
        send(
            &mut observer_writer,
            json!({"type":"subscribe","id":111,
            "working_dir":home._home.path(),"target_session_id":session_id,
            "supports_user_response":!desktop_first}),
        )
        .await;
        done(&mut observer_reader, 111).await;
        if !desktop_first {
            std::mem::swap(&mut client_reader, &mut observer_reader);
            std::mem::swap(&mut client_writer, &mut observer_writer);
            std::mem::swap(&mut server_task, &mut observer_task);
        }
        send(&mut client_writer, json!({"type":"list_tools","id":112})).await;
        let ServerEvent::Tools { tools, .. } = until(&mut client_reader, |e| {
            matches!(e, ServerEvent::Tools { id: 112, .. })
        })
        .await
        else {
            panic!("expected Desktop tool list")
        };
        assert!(
            tools.iter().any(|tool| tool.name == "user_response"),
            "an attached observer must not disable the Desktop response tool"
        );

        if clear_shared {
            let desktop_agent = sessions.read().await[&session_id].clone();
            let old_queue = soft_interrupt_queues.read().await[&session_id].clone();
            send(
                &mut client_writer,
                json!({"type":"message","id":901,
                "content":"before-clear-context","no_reply":true}),
            )
            .await;
            until(&mut client_reader, |e| {
                matches!(e, ServerEvent::ContextMessageAdded { id: 901 })
            })
            .await;
            send(&mut observer_writer, json!({"type":"clear","id":113})).await;
            let ServerEvent::SessionId {
                session_id: cleared_id,
            } = until(
                &mut observer_reader,
                |e| matches!(e, ServerEvent::SessionId { session_id: id } if id != &session_id),
            )
            .await
            else {
                panic!("expected cleared session ID")
            };
            done(&mut observer_reader, 113).await;
            response_tool(&mut client_reader, &mut client_writer, true).await;
            response_tool(&mut observer_reader, &mut observer_writer, false).await;
            let live = sessions.read().await;
            assert!(Arc::ptr_eq(&live[&session_id], &desktop_agent));
            assert!(!Arc::ptr_eq(&live[&cleared_id], &desktop_agent));
            drop(live);
            assert_eq!(desktop_agent.lock().await.session_id(), session_id);
            let attached = client_connections.read().await;
            assert_eq!(
                attached
                    .values()
                    .filter(|info| info.session_id == session_id)
                    .count(),
                1
            );
            assert_eq!(
                attached
                    .values()
                    .filter(|info| info.session_id == cleared_id)
                    .count(),
                1
            );
            drop(attached);
            assert!(Arc::ptr_eq(
                &soft_interrupt_queues.read().await[&session_id],
                &old_queue
            ));
            assert!(!Arc::ptr_eq(
                &soft_interrupt_queues.read().await[&cleared_id],
                &old_queue
            ));
            for (reader, writer, expected_id, expected_context) in [
                (&mut client_reader, &mut client_writer, &session_id, true),
                (
                    &mut observer_reader,
                    &mut observer_writer,
                    &cleared_id,
                    false,
                ),
            ] {
                send(writer, json!({"type":"get_history","id":902})).await;
                let ServerEvent::History {
                    session_id: actual,
                    messages,
                    ..
                } = until(reader, |e| {
                    matches!(e, ServerEvent::History { id: 902, .. })
                })
                .await
                else {
                    panic!("expected history")
                };
                assert_eq!(&actual, expected_id);
                assert_eq!(
                    messages
                        .iter()
                        .any(|m| m.content.contains("before-clear-context")),
                    expected_context
                );
                send(writer, json!({"type":"state","id":903})).await;
                let ServerEvent::State {
                    session_id: actual, ..
                } = until(reader, |e| matches!(e, ServerEvent::State { id: 903, .. })).await
                else {
                    panic!("expected state")
                };
                assert_eq!(&actual, expected_id);
            }
            for enabled in [true, false] {
                send(
                    &mut observer_writer,
                    json!({"type":"subscribe","id":115,
                    "working_dir":home._home.path(),"supports_user_response":enabled}),
                )
                .await;
                done(&mut observer_reader, 115).await;
                response_tool(&mut observer_reader, &mut observer_writer, enabled).await;
                response_tool(&mut client_reader, &mut client_writer, true).await;
            }
            for enabled in [false, true] {
                send(
                    &mut client_writer,
                    json!({"type":"subscribe","id":114,
                    "working_dir":home._home.path(),"supports_user_response":enabled}),
                )
                .await;
                done(&mut client_reader, 114).await;
                response_tool(&mut client_reader, &mut client_writer, enabled).await;
                response_tool(&mut observer_reader, &mut observer_writer, false).await;
            }
            for (reader, writer, marker) in [
                (
                    &mut client_reader,
                    &mut client_writer,
                    "desktop-after-clear",
                ),
                (
                    &mut observer_reader,
                    &mut observer_writer,
                    "observer-after-clear",
                ),
            ] {
                send(
                    writer,
                    json!({"type":"message","id":904,"content":marker,"no_reply":true}),
                )
                .await;
                until(reader, |e| {
                    matches!(e, ServerEvent::ContextMessageAdded { id: 904 })
                })
                .await;
            }
            for (reader, writer, own, other) in [
                (
                    &mut client_reader,
                    &mut client_writer,
                    "desktop-after-clear",
                    "observer-after-clear",
                ),
                (
                    &mut observer_reader,
                    &mut observer_writer,
                    "observer-after-clear",
                    "desktop-after-clear",
                ),
            ] {
                send(writer, json!({"type":"get_history","id":905})).await;
                let ServerEvent::History { messages, .. } = until(reader, |e| {
                    matches!(e, ServerEvent::History { id: 905, .. })
                })
                .await
                else {
                    panic!("expected isolated history")
                };
                assert!(messages.iter().any(|m| m.content.contains(own)));
                assert!(!messages.iter().any(|m| m.content.contains(other)));
            }
            // EOF of the cleared observer must not close or rebind Desktop.
            sessions.write().await.remove(&cleared_id);
            drop(observer_reader);
            drop(observer_writer);
            tokio::time::timeout(Duration::from_secs(5), observer_task)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            response_tool(&mut client_reader, &mut client_writer, true).await;
            sessions.write().await.clear();
            drop(client_reader);
            drop(client_writer);
            tokio::time::timeout(Duration::from_secs(5), server_task)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            return;
        }

        // Repeated false snapshots cannot override a supporter. With two
        // supporters, downgrading one retains the tool until the last downgrades.
        for (index, (desktop, capability, expected)) in [
            (false, false, true),
            (false, false, true),
            (true, true, true),
            (false, true, true),
            (true, false, true),
            (false, false, false),
            (true, true, true),
        ]
        .into_iter()
        .enumerate()
        {
            let id = 120 + index as u64;
            let (reader, writer) = if desktop {
                (&mut client_reader, &mut client_writer)
            } else {
                (&mut observer_reader, &mut observer_writer)
            };
            send(
                writer,
                json!({"type":"subscribe","id":id,
                "working_dir":home._home.path(),"supports_user_response":capability}),
            )
            .await;
            done(reader, id).await;
            response_tool(&mut client_reader, &mut client_writer, expected).await;
        }
        // An actual observer EOF must leave Desktop's capability enabled.
        drop(observer_writer);
        drop(observer_reader);
        tokio::time::timeout(Duration::from_secs(5), observer_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        response_tool(&mut client_reader, &mut client_writer, true).await;
        let (observer_task, observer_stream) = connect();
        let (observer_reader, mut observer_writer) = observer_stream.into_split();
        let mut observer_reader = BufReader::new(observer_reader);
        send(
            &mut observer_writer,
            json!({"type":"subscribe","id":130,
            "working_dir":home._home.path(),"target_session_id":session_id}),
        )
        .await;
        done(&mut observer_reader, 130).await;

        // Keep an observer on each live session while Desktop switches both ways.
        let (other_task, other_stream) = connect();
        let (other_reader, mut other_writer) = other_stream.into_split();
        let mut other_reader = BufReader::new(other_reader);
        send(
            &mut other_writer,
            json!({"type":"subscribe","id":131,
            "working_dir":home._home.path()}),
        )
        .await;
        done(&mut other_reader, 131).await;
        let other_id = sessions
            .read()
            .await
            .keys()
            .find(|id| *id != &session_id)
            .unwrap()
            .clone();
        send(
            &mut client_writer,
            json!({"type":"subscribe","id":132,
            "working_dir":home._home.path(),"target_session_id":other_id,
            "supports_user_response":true}),
        )
        .await;
        done(&mut client_reader, 132).await;
        response_tool(&mut observer_reader, &mut observer_writer, false).await;
        response_tool(&mut other_reader, &mut other_writer, true).await;
        send(
            &mut client_writer,
            json!({"type":"resume_session","id":133,"session_id":session_id}),
        )
        .await;
        done(&mut client_reader, 133).await;
        response_tool(&mut observer_reader, &mut observer_writer, true).await;
        response_tool(&mut other_reader, &mut other_writer, false).await;

        // EOF of the last supporter disables a session kept alive by an observer.
        drop(client_writer);
        drop(client_reader);
        tokio::time::timeout(Duration::from_secs(5), server_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        response_tool(&mut observer_reader, &mut observer_writer, false).await;
        // Catalog gating, not the unrelated 30-second idle reconnect retention.
        sessions.write().await.clear();
        drop(observer_writer);
        drop(observer_reader);
        tokio::time::timeout(Duration::from_secs(5), observer_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        drop(other_writer);
        drop(other_reader);
        tokio::time::timeout(Duration::from_secs(5), other_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        return;
    }
    send(&mut client_writer, json!({"type":"configure_tools","id":2,"tools":{"enabled":[],"custom":[{"name":"read","description":"SDK read callback","parameters":{"type":"object"}}]}})).await;
    let configured = until(&mut client_reader, |e| {
        matches!(
            e,
            ServerEvent::Ack { id: 2 } | ServerEvent::Error { id: 2, .. }
        )
    })
    .await;
    assert!(
        matches!(configured, ServerEvent::Ack { .. }),
        "{configured:?}"
    );
    send(&mut client_writer, json!({"type":"list_tools","id":3})).await;
    let ServerEvent::Tools { tools, .. } = until(&mut client_reader, |e| {
        matches!(
            e,
            ServerEvent::Tools { id: 3, .. } | ServerEvent::Error { id: 3, .. }
        )
    })
    .await
    else {
        panic!("expected tools")
    };
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read");
    send(
        &mut client_writer,
        json!({"type":"message","id":4,"content":"call read"}),
    )
    .await;
    let ServerEvent::ToolCall { call_id, name, .. } = until(&mut client_reader, |e| {
        matches!(e, ServerEvent::ToolCall { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert_eq!(name, "read");
    send(
        &mut client_writer,
        json!({"type":"configure_tools","id":5,"tools":{}}),
    )
    .await;
    let busy = until(&mut client_reader, |e| {
        matches!(
            e,
            ServerEvent::Ack { id: 5 } | ServerEvent::Error { id: 5, .. }
        )
    })
    .await;
    assert!(
        matches!(busy, ServerEvent::Error { .. }),
        "must not ack invalid/busy config: {busy:?}"
    );
    send(
        &mut client_writer,
        json!({"type":"tool_result","id":6,"call_id":call_id,"output":"SDK read output"}),
    )
    .await;
    let acknowledged = until(&mut client_reader, |e| {
        matches!(
            e,
            ServerEvent::Ack { id: 6 } | ServerEvent::Error { id: 6, .. }
        )
    })
    .await;
    assert!(
        matches!(acknowledged, ServerEvent::Ack { .. }),
        "{acknowledged:?}"
    );
    until(&mut client_reader, |e| {
        matches!(e, ServerEvent::Done { id: 4 })
    })
    .await;

    let session_a = sessions.read().await.keys().next().unwrap().clone();
    let (observer_task, observer_stream) = connect();
    let (observer_reader, mut observer_writer) = observer_stream.into_split();
    let mut observer_reader = BufReader::new(observer_reader);
    send(&mut observer_writer, json!({"type":"subscribe","id":20,"working_dir":home._home.path(),"target_session_id":session_a})).await;
    until(&mut observer_reader, |e| {
        matches!(e, ServerEvent::Done { id: 20 })
    })
    .await;

    // Explicit detach releases SDK ownership even while the socket stays open.
    send(
        &mut client_writer,
        json!({"type":"prepare_disconnect","id":21}),
    )
    .await;
    until(&mut client_reader, |e| {
        matches!(e, ServerEvent::Done { id: 21 })
    })
    .await;
    let tools = json!({"enabled":[],"custom":[{"name":"read","description":"SDK read callback","parameters":{"type":"object"}}]});
    send(
        &mut observer_writer,
        json!({"type":"configure_tools","id":22,"tools":tools}),
    )
    .await;
    let ack = until(&mut observer_reader, |e| {
        matches!(
            e,
            ServerEvent::Ack { id: 22 } | ServerEvent::Error { id: 22, .. }
        )
    })
    .await;
    assert!(
        matches!(ack, ServerEvent::Ack { .. }),
        "detach must release owner: {ack:?}"
    );

    // Switching the owner to B must leave A unavailable, not route A's
    // callback to B. The first client keeps A's Agent alive throughout.
    let mut other_session =
        crate::session::Session::create_with_id("sdk_other_session".into(), None, None);
    other_session.working_dir = Some(home._home.path().to_string_lossy().into_owned());
    other_session.save_prepared().unwrap();
    assert!(crate::session::session_exists(&other_session.id));
    let other_registry = Registry::new(provider_template.clone()).await;
    sessions.write().await.insert(
        "sdk_other_session".into(),
        Arc::new(Mutex::new(Agent::new_with_session(
            provider_template.clone(),
            other_registry,
            other_session,
            None,
        ))),
    );
    send(
        &mut observer_writer,
        json!({"type":"resume_session","id":23,"session_id":"sdk_other_session"}),
    )
    .await;
    until(&mut observer_reader, |e| {
        matches!(e, ServerEvent::Done { id: 23 })
    })
    .await;
    send(
        &mut client_writer,
        json!({"type":"message","id":24,"content":"call read after owner switch"}),
    )
    .await;
    until(&mut client_reader, |e| {
        assert!(
            !matches!(e, ServerEvent::ToolCall { .. }),
            "disconnected owner must not receive callback"
        );
        matches!(e, ServerEvent::Done { id: 24 })
    })
    .await;
    send(&mut observer_writer, json!({"type":"list_tools","id":25})).await;
    until(&mut observer_reader, |e| {
        assert!(
            !matches!(e, ServerEvent::ToolCall { .. }),
            "A callback must never reach B's owner connection"
        );
        matches!(e, ServerEvent::Tools { id: 25, .. })
    })
    .await;

    // A new owner can configure retained A after the prior owner switches.
    send(
        &mut client_writer,
        json!({"type":"configure_tools","id":26,"tools":tools}),
    )
    .await;
    let ack = until(&mut client_reader, |e| {
        matches!(
            e,
            ServerEvent::Ack { id: 26 } | ServerEvent::Error { id: 26, .. }
        )
    })
    .await;
    assert!(
        matches!(ack, ServerEvent::Ack { .. }),
        "session switch must release owner: {ack:?}"
    );
    send(
        &mut client_writer,
        json!({"type":"message","id":7,"content":"call read again"}),
    )
    .await;
    let event = until(&mut client_reader, |e| {
        matches!(e, ServerEvent::ToolCall { .. })
    })
    .await;
    assert!(matches!(event, ServerEvent::ToolCall { session_id, .. } if session_id == session_a));
    drop(observer_writer);
    drop(observer_reader);
    tokio::time::timeout(Duration::from_secs(5), observer_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    // Disconnect with a pending callback must not wait for the 120s deadline.
    drop(client_writer);
    drop(client_reader);
    tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
