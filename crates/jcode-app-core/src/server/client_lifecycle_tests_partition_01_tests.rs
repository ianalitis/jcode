#[test]
fn client_initiated_turn_fans_out_stream_and_terminal_events_to_live_attachments() {
    let _guard = crate::storage::lock_test_env();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build current-thread test runtime")
        .block_on(async {
            let _runtime = IsolatedRuntimeDir::new();
            let session_id = "session_live_attachment_fanout";

            let provider: Arc<dyn Provider> = Arc::new(FanoutStreamProvider);
            let registry = Registry::new(Arc::clone(&provider)).await;
            let mut session =
                crate::session::Session::create_with_id(session_id.to_string(), None, None);
            session.model = Some("fanout-stream".to_string());
            let agent = Arc::new(Mutex::new(Agent::new_with_session(
                provider, registry, session, None,
            )));

            let (origin_tx, mut origin_rx) = mpsc::unbounded_channel::<ServerEvent>();
            let (attached_tx, mut attached_rx) = mpsc::unbounded_channel::<ServerEvent>();
            let swarm_members = Arc::new(RwLock::new(HashMap::from([(
                session_id.to_string(),
                SwarmMember {
                    session_id: session_id.to_string(),
                    event_tx: origin_tx.clone(),
                    event_txs: HashMap::from([("origin".to_string(), origin_tx.clone())]),
                    working_dir: None,
                    swarm_id: None,
                    swarm_enabled: false,
                    status: "ready".to_string(),
                    detail: None,
                    task_label: None,
                    friendly_name: None,
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
                },
            )])));
            let swarms_by_id = Arc::new(RwLock::new(HashMap::new()));
            let event_history = Arc::new(RwLock::new(std::collections::VecDeque::new()));
            let event_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let (swarm_event_tx, _) = broadcast::channel(8);
            let (processing_done_tx, mut processing_done_rx) = mpsc::unbounded_channel();
            let mut client_is_processing = false;
            let mut processing_message_id = None;
            let mut processing_session_id = None;
            let mut processing_task = None;

            start_processing_message(
                ProcessingMessage {
                    id: 479,
                    content: "stream to every attachment".to_string(),
                    images: Vec::new(),
                    system_reminder: None,
                    active_skill: None,
                },
                session_id,
                &mut ProcessingState {
                    client_is_processing: &mut client_is_processing,
                    message_id: &mut processing_message_id,
                    session_id: &mut processing_session_id,
                    task: &mut processing_task,
                },
                &agent,
                &origin_tx,
                &processing_done_tx,
                Vec::new(),
                &SwarmStatusRefs {
                    members: &swarm_members,
                    swarms_by_id: &swarms_by_id,
                    event_history: &event_history,
                    event_counter: &event_counter,
                    event_tx: &swarm_event_tx,
                },
            )
            .await;

            loop {
                let event = tokio::time::timeout(Duration::from_secs(2), origin_rx.recv())
                    .await
                    .expect("origin should receive the initial stream event promptly")
                    .expect("origin event channel should remain open");
                if matches!(event, ServerEvent::TextDelta { ref text } if text == "before attach") {
                    break;
                }
            }

            crate::server::register_session_event_sender(
                &swarm_members,
                session_id,
                "attached",
                attached_tx,
            )
            .await;

            for rx in [&mut origin_rx, &mut attached_rx] {
                let mut saw_post_attach_delta = false;
                let mut saw_message_end = false;
                let mut saw_done = false;
                while !saw_post_attach_delta || !saw_message_end || !saw_done {
                    let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                        .await
                        .expect("attachment should receive streamed event promptly")
                        .expect("attachment event channel should remain open");
                    saw_post_attach_delta |= matches!(
                        event,
                        ServerEvent::TextDelta { ref text } if text == "after attach"
                    );
                    if matches!(event, ServerEvent::MessageEnd { .. }) {
                        assert!(!saw_done, "MessageEnd must precede the terminal Done event");
                        saw_message_end = true;
                    }
                    saw_done |= matches!(event, ServerEvent::Done { id: 479 });
                }
            }

            let (done_id, result, _) =
                tokio::time::timeout(Duration::from_secs(2), processing_done_rx.recv())
                    .await
                    .expect("processing should complete promptly")
                    .expect("processing completion channel should remain open");
            assert_eq!(done_id, 479);
            result.expect("turn should complete successfully");

            if let Some(handle) = processing_task.take() {
                handle.await.expect("processing task join");
            }
        });
}

#[test]
fn accepted_reload_recovery_continuation_marks_intent_delivered() -> anyhow::Result<()> {
    let _lock = crate::storage::lock_test_env();
    let _env = IsolatedReloadRecoveryEnv::new();
    let session_id = "session_accepted_reload_recovery";
    let continuation = "stored continuation accepted by server";

    super::super::reload_recovery::persist_intent(
        "reload-accepted-continuation",
        session_id,
        super::super::reload_recovery::ReloadRecoveryRole::InterruptedPeer,
        crate::tool::selfdev::ReloadRecoveryDirective {
            reconnect_notice: Some("stored notice".to_string()),
            continuation_message: continuation.to_string(),
        },
        "synthetic accepted continuation test",
    )?;
    assert!(super::super::reload_recovery::has_pending_for_session(
        session_id
    ));

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let provider: Arc<dyn Provider> = Arc::new(CompleteImmediatelyProvider);
        let registry = Registry::new(Arc::clone(&provider)).await;
        let mut session =
            crate::session::Session::create_with_id(session_id.to_string(), None, None);
        session.model = Some("complete-immediately".to_string());
        let agent = Arc::new(Mutex::new(Agent::new_with_session(
            provider, registry, session, None,
        )));

        let (client_event_tx, _client_event_rx) = mpsc::unbounded_channel::<ServerEvent>();
        let (processing_done_tx, mut processing_done_rx) = mpsc::unbounded_channel();
        let mut client_is_processing = false;
        let mut processing_message_id = None;
        let mut processing_session_id = None;
        let mut processing_task = None;
        let swarm_members = Arc::new(RwLock::new(HashMap::new()));
        let swarms_by_id = Arc::new(RwLock::new(HashMap::new()));
        let event_history = Arc::new(RwLock::new(std::collections::VecDeque::new()));
        let event_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let (swarm_event_tx, _) = broadcast::channel(8);

        start_processing_message(
            ProcessingMessage {
                id: 77,
                content: "continue after reload".to_string(),
                images: Vec::new(),
                system_reminder: Some(continuation.to_string()),
                active_skill: None,
            },
            session_id,
            &mut ProcessingState {
                client_is_processing: &mut client_is_processing,
                message_id: &mut processing_message_id,
                session_id: &mut processing_session_id,
                task: &mut processing_task,
            },
            &agent,
            &client_event_tx,
            &processing_done_tx,
            Vec::new(),
            &SwarmStatusRefs {
                members: &swarm_members,
                swarms_by_id: &swarms_by_id,
                event_history: &event_history,
                event_counter: &event_counter,
                event_tx: &swarm_event_tx,
            },
        )
        .await;

        assert!(client_is_processing);
        assert_eq!(processing_message_id, Some(77));
        assert_eq!(processing_session_id.as_deref(), Some(session_id));
        assert!(processing_task.is_some());
        assert!(
            !super::super::reload_recovery::has_pending_for_session(session_id),
            "server acceptance of the exact hidden continuation should consume the durable intent"
        );

        let (done_id, result, _report) =
            tokio::time::timeout(std::time::Duration::from_secs(5), processing_done_rx.recv())
                .await
                .expect("processing task should finish")
                .expect("processing task should report completion");
        assert_eq!(done_id, 77);
        result?;
        if let Some(handle) = processing_task.take() {
            handle.await.expect("processing task join");
        }
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

#[test]
fn reload_starting_rejects_new_turns_for_multiple_sessions() {
    let _guard = crate::storage::lock_test_env();
    let _runtime = IsolatedRuntimeDir::new();
    crate::server::write_reload_state(
        "reload-lifecycle-multi-starting",
        "test-hash",
        crate::server::ReloadPhase::Starting,
        Some("session_alpha".to_string()),
    );

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let forked = Arc::new(AtomicBool::new(false));
        let provider: Arc<dyn Provider> = Arc::new(PanicOnForkProvider {
            forked: Arc::clone(&forked),
        });
        let registry = Registry::new(Arc::clone(&provider)).await;
        let swarm_members = Arc::new(RwLock::new(HashMap::new()));
        let swarms_by_id = Arc::new(RwLock::new(HashMap::new()));
        let event_history = Arc::new(RwLock::new(std::collections::VecDeque::new()));
        let event_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let (swarm_event_tx, _) = broadcast::channel(8);

        for (message_id, session_id) in [
            (101, "session_alpha"),
            (102, "session_beta"),
            (103, "session_gamma"),
        ] {
            let mut session =
                crate::session::Session::create_with_id(session_id.to_string(), None, None);
            session.model = Some("panic-on-fork".to_string());
            let agent = Arc::new(Mutex::new(Agent::new_with_session(
                Arc::clone(&provider),
                registry.clone(),
                session,
                None,
            )));

            let (client_event_tx, mut client_event_rx) = mpsc::unbounded_channel::<ServerEvent>();
            let (processing_done_tx, mut processing_done_rx) = mpsc::unbounded_channel();
            let mut client_is_processing = false;
            let mut processing_message_id = None;
            let mut processing_session_id = None;
            let mut processing_task = None;

            start_processing_message(
                ProcessingMessage {
                    id: message_id,
                    content: format!("do not start {session_id} during reload"),
                    images: Vec::new(),
                    system_reminder: None,
                    active_skill: None,
                },
                session_id,
                &mut ProcessingState {
                    client_is_processing: &mut client_is_processing,
                    message_id: &mut processing_message_id,
                    session_id: &mut processing_session_id,
                    task: &mut processing_task,
                },
                &agent,
                &client_event_tx,
                &processing_done_tx,
                Vec::new(),
                &SwarmStatusRefs {
                    members: &swarm_members,
                    swarms_by_id: &swarms_by_id,
                    event_history: &event_history,
                    event_counter: &event_counter,
                    event_tx: &swarm_event_tx,
                },
            )
            .await;

            let event = tokio::time::timeout(
                std::time::Duration::from_millis(250),
                client_event_rx.recv(),
            )
            .await
            .expect("reload guard should emit promptly for every session")
            .expect("reload event should be sent to client");
            assert!(
                matches!(event, ServerEvent::Reloading { new_socket: None }),
                "expected Reloading event for {session_id}, got {event:?}"
            );
            assert!(
                client_event_rx.try_recv().is_err(),
                "reload guard should only emit one reload notification for {session_id}"
            );
            assert!(
                !client_is_processing,
                "{session_id} should not enter processing during reload"
            );
            assert_eq!(processing_message_id, None);
            assert_eq!(processing_session_id, None);
            assert!(
                processing_task.is_none(),
                "{session_id} should not spawn a processing task during reload"
            );
            assert!(processing_done_rx.try_recv().is_err());
        }

        assert!(
            !forked.load(Ordering::SeqCst),
            "rejecting multiple sessions during reload should not fork or invoke provider work"
        );
    });
}

#[tokio::test]
async fn lightweight_comm_request_skips_full_session_initialization() {
    let (server_stream, client_stream) = crate::transport::Stream::pair().expect("socket pair");
    let forked = Arc::new(AtomicBool::new(false));
    let provider_template: Arc<dyn Provider> = Arc::new(PanicOnForkProvider {
        forked: Arc::clone(&forked),
    });

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

    let server_task = tokio::spawn(handle_client(
        server_stream,
        Arc::clone(&sessions),
        _global_event_tx,
        provider_template,
        global_is_processing,
        global_session_id,
        client_count,
        Arc::clone(&client_connections),
        swarm_members,
        swarms_by_id,
        shared_context,
        swarm_plans,
        swarm_coordinators,
        file_touch,
        channel_subscriptions,
        channel_subscriptions_by_session,
        client_debug_state,
        _debug_response_tx,
        event_history,
        event_counter,
        swarm_event_tx,
        "jcode-test".to_string(),
        "🧪".to_string(),
        mcp_pool,
        shutdown_signals,
        soft_interrupt_queues,
        AwaitMembersRuntime::default(),
        SwarmMutationRuntime::default(),
    ));

    let (client_reader, mut client_writer) = client_stream.into_split();
    let mut client_reader = BufReader::new(client_reader);
    let request = Request::CommList {
        id: 7,
        session_id: "not-in-swarm".to_string(),
    };
    let payload = serde_json::to_string(&request).expect("serialize request") + "\n";
    client_writer
        .write_all(payload.as_bytes())
        .await
        .expect("write request");

    let mut line = String::new();
    client_reader
        .read_line(&mut line)
        .await
        .expect("read ack bytes");
    let ack = decode_request_or_event(&line);
    assert!(matches!(ack, ServerEvent::Ack { id: 7 }));

    line.clear();
    client_reader
        .read_line(&mut line)
        .await
        .expect("read terminal response");
    let response = decode_request_or_event(&line);
    match response {
        ServerEvent::Error { id, message, .. } => {
            assert_eq!(id, 7);
            assert!(message.contains("Not in a swarm"));
        }
        other => panic!("expected error response, got {other:?}"),
    }

    line.clear();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), client_reader.read_line(&mut line))
            .await
            .expect("non-Ping lightweight command must close its one-shot connection")
            .expect("read EOF"),
        0,
    );
    drop(client_writer);
    server_task
        .await
        .expect("server task join")
        .expect("server task result");

    assert!(
        !forked.load(Ordering::SeqCst),
        "lightweight control request should not fork a provider"
    );
    assert!(
        client_connections.read().await.is_empty(),
        "lightweight control request should not register a live client session"
    );
    assert!(
        sessions.read().await.is_empty(),
        "lightweight control request should not allocate a live agent session"
    );
}

fn decode_request_or_event(line: &str) -> ServerEvent {
    serde_json::from_str(line.trim()).expect("decode server event")
}

#[test]
fn soft_interrupt_dispatch_starts_idle_session_and_queues_busy_session() {
    assert!(should_start_idle_soft_interrupt(false, false, false));
    assert!(!should_start_idle_soft_interrupt(true, false, false));
    assert!(!should_start_idle_soft_interrupt(false, true, false));
    assert!(!should_start_idle_soft_interrupt(false, false, true));
}

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
    let (server_task, client_stream) = connect();

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
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).await.unwrap() > 0);
                let event: ServerEvent = serde_json::from_str(&line).unwrap();
                if predicate(&event) {
                    return event;
                }
            }
        })
        .await
        .expect("daemon event timeout")
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
    other_session.save().unwrap();
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

#[test]
fn subscribe_system_prompt_is_creation_only_and_preserves_empty() {
    for prompt in [None, Some("custom system prompt"), Some("")] {
        assert_eq!(new_session_system_prompt(true, None, prompt), prompt);
        assert_eq!(new_session_system_prompt(false, None, prompt), None);
        assert_eq!(
            new_session_system_prompt(true, Some("existing"), prompt),
            None
        );
        assert_eq!(
            new_session_system_prompt(false, Some("existing"), prompt),
            None
        );
    }
}

#[derive(Clone)]
struct SystemPromptCaptureProvider(Arc<std::sync::Mutex<Vec<String>>>);

#[async_trait]
impl Provider for SystemPromptCaptureProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        self.0.lock().unwrap().push(system.to_string());
        Ok(Box::pin(stream::iter(vec![
            Ok(StreamEvent::TextDelta("ok".into())),
            Ok(StreamEvent::MessageEnd { stop_reason: None }),
        ])))
    }
    fn name(&self) -> &str {
        "prompt-capture"
    }
    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(self.clone())
    }
}

#[tokio::test]
async fn system_prompt_socket_creation_attach_resume_fork_and_no_leaking() {
    let _lock = crate::storage::lock_test_env();
    let home = IsolatedReloadRecoveryEnv::new();
    let captured = Arc::new(std::sync::Mutex::new(Vec::new()));
    let provider_template: Arc<dyn Provider> =
        Arc::new(SystemPromptCaptureProvider(captured.clone()));

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
    let (server_task, client_stream) = connect();

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
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).await.unwrap() > 0);
                let event: ServerEvent = serde_json::from_str(&line).unwrap();
                assert!(
                    !matches!(event, ServerEvent::Error { .. }),
                    "unexpected server error: {event:?}"
                );
                if predicate(&event) {
                    return event;
                }
            }
        })
        .await
        .expect("daemon event timeout")
    }
    use serde_json::json;

    for prompt in ["SDK custom system prompt", ""] {
        // A fresh connection creates a session and persists its explicit prompt
        // before acknowledging Subscribe, even with no conversation messages.
        let (owner_task, owner_stream) = connect();
        let (owner_reader, mut owner_writer) = owner_stream.into_split();
        let mut owner_reader = BufReader::new(owner_reader);
        send(
            &mut owner_writer,
            json!({"type":"subscribe","id":101,
            "working_dir":home._home.path(),"system_prompt":prompt}),
        )
        .await;
        until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::Done { id: 101 })
        })
        .await;
        send(
            &mut owner_writer,
            serde_json::to_value(Request::GetState { id: 102 }).unwrap(),
        )
        .await;
        let ServerEvent::State {
            session_id: parent_id,
            ..
        } = until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::State { id: 102, .. })
        })
        .await
        else {
            unreachable!()
        };
        let saved = crate::session::Session::load(&parent_id).unwrap();
        assert_eq!(saved.system_prompt.as_deref(), Some(prompt));
        assert_eq!(saved.visible_conversation_message_count(), 0);

        // Repeated Subscribe cannot mutate even the current owner's prompt.
        send(
            &mut owner_writer,
            json!({"type":"subscribe","id":103,
            "working_dir":home._home.path(),"system_prompt":"replacement forbidden"}),
        )
        .await;
        until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::Done { id: 103 })
        })
        .await;
        assert_eq!(
            crate::session::Session::load(&parent_id)
                .unwrap()
                .system_prompt
                .as_deref(),
            Some(prompt)
        );

        // An attaching observer likewise cannot replace another session's prompt.
        send(
            &mut client_writer,
            json!({"type":"subscribe","id":104,
            "target_session_id":parent_id,"system_prompt":"attachment forbidden"}),
        )
        .await;
        until(&mut client_reader, |e| {
            matches!(e, ServerEvent::Done { id: 104 })
        })
        .await;
        send(
            &mut owner_writer,
            json!({"type":"message","id":105,"content":"hello"}),
        )
        .await;
        until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::Done { id: 105 })
        })
        .await;
        assert_eq!(
            captured.lock().unwrap().last().map(String::as_str),
            Some(prompt)
        );

        send(&mut owner_writer, json!({"type":"split","id":106})).await;
        let ServerEvent::SplitResponse {
            new_session_id: child_id,
            ..
        } = until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::SplitResponse { id: 106, .. })
        })
        .await
        else {
            unreachable!()
        };
        assert_eq!(
            crate::session::Session::load(&child_id)
                .unwrap()
                .system_prompt
                .as_deref(),
            Some(prompt)
        );
        // This attachment restores a persisted fork, rather than a live Agent.
        send(
            &mut client_writer,
            json!({"type":"subscribe","id":107,
            "target_session_id":child_id,"system_prompt":"fork replacement forbidden"}),
        )
        .await;
        until(&mut client_reader, |e| {
            matches!(e, ServerEvent::Done { id: 107 })
        })
        .await;
        send(
            &mut client_writer,
            json!({"type":"message","id":108,"content":"fork hello"}),
        )
        .await;
        until(&mut client_reader, |e| {
            matches!(e, ServerEvent::Done { id: 108 })
        })
        .await;
        assert_eq!(
            captured.lock().unwrap().last().map(String::as_str),
            Some(prompt)
        );

        send(&mut owner_writer, json!({"type":"clear","id":109})).await;
        until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::Done { id: 109 })
        })
        .await;
        send(
            &mut owner_writer,
            json!({"type":"message","id":110,"content":"new session"}),
        )
        .await;
        until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::Done { id: 110 })
        })
        .await;
        assert_ne!(
            captured.lock().unwrap().last().map(String::as_str),
            Some(prompt)
        );
        send(
            &mut owner_writer,
            json!({"type":"resume_session","id":111,"session_id":parent_id}),
        )
        .await;
        until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::Done { id: 111 })
        })
        .await;
        send(
            &mut owner_writer,
            json!({"type":"message","id":112,"content":"resumed hello"}),
        )
        .await;
        until(&mut owner_reader, |e| {
            matches!(e, ServerEvent::Done { id: 112 })
        })
        .await;
        assert_eq!(
            captured.lock().unwrap().last().map(String::as_str),
            Some(prompt)
        );

        drop(owner_writer);
        drop(owner_reader);
        tokio::time::timeout(Duration::from_secs(5), owner_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    drop(client_writer);
    drop(client_reader);
    tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
