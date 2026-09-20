#[test]
fn schema_branches_only_require_properties_they_declare() {
    // Gemini rejects the entire request when a `required` entry names a property
    // the same object does not define, which made every tool-enabled Gemini call
    // fail on this tool's spawn branch (issue #655).
    let schema = CommunicateTool::new().parameters_schema();
    for branch in schema["anyOf"].as_array().expect("schema branches") {
        let declared = branch["properties"]
            .as_object()
            .expect("branch properties")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for required in branch["required"].as_array().expect("branch required") {
            let name = required.as_str().expect("required name");
            assert!(
                declared.iter().any(|known| known == name),
                "branch requires '{name}' without declaring it: {branch}"
            );
        }
    }
}

#[test]
fn spawn_label_validation_rejects_missing_or_blank_labels() {
    let missing: CommunicateInput =
        serde_json::from_value(json!({"action": "spawn"})).expect("spawn input");
    assert_eq!(
        missing
            .required_spawn_label()
            .expect_err("missing label must fail")
            .to_string(),
        "'label' is required for spawn action"
    );

    let blank: CommunicateInput = serde_json::from_value(json!({
        "action": "spawn",
        "label": "  \n\t "
    }))
    .expect("spawn input");
    assert_eq!(
        blank
            .required_spawn_label()
            .expect_err("blank label must fail")
            .to_string(),
        "'label' must not be blank for spawn action"
    );
}

#[test]
fn spawn_label_validation_trims_valid_labels() {
    let params: CommunicateInput = serde_json::from_value(json!({
        "action": "spawn",
        "label": "  api reviewer  "
    }))
    .expect("spawn input");
    assert_eq!(
        params.required_spawn_label().expect("valid label"),
        "api reviewer"
    );
}

#[tokio::test]
async fn spawn_execute_rejects_missing_label_before_sending_request() {
    let working_dir = tempfile::tempdir().expect("working dir");
    let error = CommunicateTool::new()
        .execute(
            json!({"action": "spawn", "prompt": "review the API"}),
            test_ctx("session-parent", working_dir.path()),
        )
        .await
        .expect_err("missing spawn label must fail locally");

    assert_eq!(error.to_string(), "'label' is required for spawn action");
}

#[test]
fn description_includes_swarm_prompt_guidance() {
    let tool = CommunicateTool::new();
    let description = tool.description();
    assert!(
        description.starts_with("Coordinate agents"),
        "description should lead with the short coordination summary"
    );
    assert!(
        description.contains("Swarm prompt"),
        "description should embed the swarm prompt section"
    );
}

#[test]
fn existing_tool_keeps_prompt_while_new_tool_loads_edit() {
    let project = tempfile::tempdir().unwrap();
    let prompt_dir = project.path().join(".jcode");
    std::fs::create_dir_all(&prompt_dir).unwrap();
    let prompt_path = prompt_dir.join("swarm-prompt.md");
    std::fs::write(&prompt_path, "first routing version").unwrap();

    let existing = CommunicateTool::new_for_working_dir(Some(project.path()));
    std::fs::write(&prompt_path, "second routing version").unwrap();
    let newly_created = CommunicateTool::new_for_working_dir(Some(project.path()));

    assert!(existing.description().contains("first routing version"));
    assert!(!existing.description().contains("second routing version"));
    assert!(
        newly_created
            .description()
            .contains("second routing version")
    );
}

#[test]
fn spawning_action_inputs_preserve_requested_model() {
    for action in [
        "spawn",
        "assign_task",
        "assign_next",
        "fill_slots",
        "run_plan",
    ] {
        for model in [
            "z-ai/glm-5.2:free",
            "openai-api:gpt-5.5",
            "inherit",
            "coordinator",
            "  ",
        ] {
            let input: CommunicateInput = serde_json::from_value(json!({
                "action": action,
                "label": "reviewer",
                "model": model
            }))
            .unwrap();
            assert_eq!(input.model.as_deref(), Some(model));
        }
    }
}

#[test]
fn spawning_action_inputs_allow_omitted_or_null_model() {
    for action in [
        "spawn",
        "assign_task",
        "assign_next",
        "fill_slots",
        "run_plan",
    ] {
        let without_model: CommunicateInput =
            serde_json::from_value(json!({"action": action, "label": "reviewer"})).unwrap();
        assert!(without_model.model.is_none());
        let null_model: CommunicateInput = serde_json::from_value(json!({
            "action": action,
            "label": "reviewer",
            "model": null
        }))
        .unwrap();
        assert!(null_model.model.is_none());
    }
}

#[test]
fn format_swarm_model_list_renders_routes_and_default() {
    let routes = vec![
        jcode_provider_core::ModelRoute {
            model: "gpt-5.5".to_string(),
            provider: "OpenAI".to_string(),
            api_method: "openai-api-key".to_string(),
            available: true,
            detail: "API key".to_string(),
            usage: None,
            cheapness: None,
        },
        jcode_provider_core::ModelRoute {
            model: "claude-fable-5".to_string(),
            provider: "Anthropic".to_string(),
            api_method: "anthropic-api-key".to_string(),
            available: false,
            detail: String::new(),
            usage: None,
            cheapness: None,
        },
    ];
    let output =
        format_swarm_model_list(Some("claude-fable-5"), Some("openai-api:gpt-5.5"), &routes, None);
    assert!(output.contains("Current coordinator model: claude-fable-5"));
    assert!(output.contains("Configured agents.swarm_model default: openai-api:gpt-5.5"));
    assert!(output.contains("gpt-5.5 via OpenAI [openai-api-key] (API key)"));
    assert!(output.contains("claude-fable-5 via Anthropic [anthropic-api-key] [unavailable]"));
    assert!(output.contains("effort"));
}

#[test]
fn format_swarm_model_list_filters_by_query_and_caps_detail() {
    let route = |model: &str, provider: &str, api_method: &str| jcode_provider_core::ModelRoute {
        model: model.to_string(),
        provider: provider.to_string(),
        api_method: api_method.to_string(),
        available: true,
        detail: String::new(),
        usage: None,
        cheapness: None,
    };
    let routes = vec![
        route("gpt-5.6-terra", "OpenAI", "openai-oauth"),
        route("claude-opus-5", "Anthropic", "claude-oauth"),
        route("deepseek-v4.1-flash", "OpenCode Go", "openai-compatible"),
    ];

    let filtered = format_swarm_model_list(None, None, &routes, Some("opencode go"));
    assert!(filtered.contains("Model routes matching \"opencode go\" (1 of 3)"));
    assert!(filtered.contains("deepseek-v4.1-flash via OpenCode Go"));
    assert!(!filtered.contains("gpt-5.6-terra via OpenAI"));

    let by_api = format_swarm_model_list(None, None, &routes, Some("oauth"));
    assert!(by_api.contains("(2 of 3)"));

    let none = format_swarm_model_list(None, None, &routes, Some("no-such-provider"));
    assert!(none.contains("(0 of 3)"));
    assert!(none.contains("none. Try a shorter query"));

    // A large catalog must not dump every route into agent context.
    let many: Vec<_> = (0..200)
        .map(|index| route(&format!("model-{index}"), "Provider", "openai-compatible"))
        .collect();
    let capped = format_swarm_model_list(None, None, &many, None);
    assert!(capped.contains("200 total, 200 available"));
    assert!(capped.contains("140 more matching route(s) omitted"));
    assert!(capped.contains("model-59"));
    assert!(!capped.contains("model-60 "));
}

#[test]
fn format_swarm_model_list_lists_available_routes_before_unavailable_ones() {
    let route = |model: &str, available: bool| jcode_provider_core::ModelRoute {
        model: model.to_string(),
        provider: "Provider".to_string(),
        api_method: "openai-compatible".to_string(),
        available,
        detail: String::new(),
        usage: None,
        cheapness: None,
    };
    let routes = vec![
        route("unavailable-a", false),
        route("available-b", true),
        route("unavailable-c", false),
        route("available-d", true),
    ];

    let output = format_swarm_model_list(None, None, &routes, None);
    let available_at = output.find("available-b").expect("available route listed");
    let unavailable_at = output
        .find("unavailable-a")
        .expect("unavailable route listed past the cap");
    assert!(
        available_at < unavailable_at,
        "available routes must precede unavailable ones inside the detail limit"
    );
    assert!(output.contains("4 total, 2 available"));
}

#[test]
fn format_swarm_model_list_handles_empty_catalog() {
    let output = format_swarm_model_list(None, None, &[], None);
    assert!(output.contains("Current coordinator model: unknown"));
    assert!(output.contains("No agents.swarm_model default configured"));
    assert!(output.contains("unless model is passed"));
    assert!(output.contains("No model routes reported"));
}

#[test]
fn schema_advertises_supported_swarm_fields() {
    let schema = CommunicateTool::new().parameters_schema();
    let props = schema["properties"]
        .as_object()
        .expect("swarm schema should have properties");

    assert!(props.contains_key("action"));
    assert!(props.contains_key("key"));
    assert!(props.contains_key("value"));
    assert!(props.contains_key("message"));
    assert!(props.contains_key("to_session"));
    assert_eq!(
        props["to_session"]["description"],
        json!("Session ID or unique friendly name of one agent. Alias of target_session.")
    );
    assert!(props.contains_key("channel"));
    assert!(props.contains_key("proposer_session"));
    assert!(props.contains_key("reason"));
    assert!(props.contains_key("target_session"));
    assert_eq!(
        props["target_session"]["description"],
        json!("Session ID or unique friendly name for management actions. Alias of to_session.")
    );
    assert!(props.contains_key("role"));
    assert!(props.contains_key("prompt"));
    assert!(props.contains_key("working_dir"));
    assert!(props.contains_key("limit"));
    assert!(props.contains_key("task_id"));
    assert!(props.contains_key("spawn_if_needed"));
    assert!(props.contains_key("prefer_spawn"));
    assert!(props.contains_key("session_ids"));
    assert!(props.contains_key("mode"));
    assert_eq!(
        props["mode"]["enum"],
        json!(["all", "any", "deep", "light"]),
        "mode must advertise both task_graph and await_members values"
    );
    assert!(props.contains_key("target_status"));
    assert!(props.contains_key("timeout_minutes"));
    assert!(props.contains_key("concurrency_limit"));
    assert!(props.contains_key("wake"));
    assert!(props.contains_key("delivery"));
    assert!(props.contains_key("plan_items"));
    assert!(props.contains_key("initial_message"));
    assert!(props.contains_key("force"));
    assert!(props.contains_key("retain_agents"));
    assert!(props.contains_key("background"));
    assert!(
        props["background"]["description"]
            .as_str()
            .expect("background description")
            .contains("run_plan"),
        "background flag should document run_plan support"
    );
    assert!(props.contains_key("notify"));
    assert!(props.contains_key("status"));
    assert!(props.contains_key("validation"));
    assert!(props.contains_key("follow_up"));
    assert_eq!(
        props["delivery"]["enum"],
        json!(["notify", "interrupt", "wake"])
    );
    assert_eq!(
        props["plan_items"]["items"]["additionalProperties"],
        json!(true)
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("status"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("report"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("plan_status"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("start"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("start_task"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("assign_next"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("fill_slots"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("run_plan"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("cleanup"))
    );
    assert!(
        schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum")
            .contains(&json!("salvage"))
    );
}

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let original = std::env::var_os(key);
        crate::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = self.original.take() {
            crate::env::set_var(self.key, value);
        } else {
            crate::env::remove_var(self.key);
        }
    }
}

struct DelayedTestProvider {
    delay: Duration,
}

#[async_trait]
impl Provider for DelayedTestProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let delay = self.delay;
        let stream = futures::stream::once(async move {
            tokio::time::sleep(delay).await;
            Ok(StreamEvent::TextDelta("ok".to_string()))
        })
        .chain(futures::stream::once(async {
            Ok(StreamEvent::MessageEnd { stop_reason: None })
        }));
        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "test"
    }

    fn declared_spawn_route_class(&self) -> Option<jcode_attempt_types::RouteClass> {
        // In-process stub: never leaves the test, so spawn admission may
        // classify it as Local instead of failing closed on the `test` key.
        Some(jcode_attempt_types::RouteClass::Local)
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self { delay: self.delay })
    }
}

struct RawClient {
    reader: BufReader<ReadHalf>,
    writer: WriteHalf,
    next_id: u64,
}

impl RawClient {
    async fn connect(path: &Path) -> Result<Self> {
        let stream = Stream::connect(path).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader: BufReader::new(reader),
            writer,
            next_id: 1,
        })
    }

    async fn send_request(&mut self, request: Request) -> Result<u64> {
        let id = request.id();
        let json = serde_json::to_string(&request)? + "\n";
        self.writer.write_all(json.as_bytes()).await?;
        Ok(id)
    }

    async fn read_event(&mut self) -> Result<ServerEvent> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            anyhow::bail!("server disconnected")
        }
        Ok(serde_json::from_str(&line)?)
    }

    async fn read_until<F>(&mut self, timeout: Duration, mut predicate: F) -> Result<ServerEvent>
    where
        F: FnMut(&ServerEvent) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let event = tokio::time::timeout(remaining, self.read_event()).await??;
            if predicate(&event) {
                return Ok(event);
            }
        }
    }

    async fn subscribe(&mut self, working_dir: &Path) -> Result<()> {
        self.subscribe_to(working_dir, None).await
    }

    async fn subscribe_to(
        &mut self,
        working_dir: &Path,
        target_session_id: Option<&str>,
    ) -> Result<()> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_request(Request::Subscribe {
            id,
            working_dir: Some(working_dir.display().to_string()),
            selfdev: None,
            target_session_id: target_session_id.map(str::to_string),
            client_instance_id: None,
            client_has_local_history: false,
            allow_session_takeover: false,
            crash_on_disconnect: false,
            continue_on_disconnect: false,
            terminal_env: Vec::new(),
        })
        .await?;
        self.read_until(
            Duration::from_secs(5),
            |event| matches!(event, ServerEvent::Done { id: done_id } if *done_id == id),
        )
        .await?;
        Ok(())
    }

    async fn session_id(&mut self) -> Result<String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_request(Request::GetState { id }).await?;
        match self
            .read_until(
                Duration::from_secs(5),
                |event| matches!(event, ServerEvent::State { id: event_id, .. } if *event_id == id),
            )
            .await?
        {
            ServerEvent::State { session_id, .. } => Ok(session_id),
            other => anyhow::bail!("unexpected state response: {other:?}"),
        }
    }

    async fn send_message(&mut self, content: &str) -> Result<u64> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_request(Request::Message {
            id,
            content: content.to_string(),
            images: vec![],
            system_reminder: None,
            active_skill: None,
            no_reply: false,
        })
        .await
    }

    async fn wait_for_done(&mut self, request_id: u64) -> Result<()> {
        self.read_until(
            Duration::from_secs(10),
            |event| matches!(event, ServerEvent::Done { id } if *id == request_id),
        )
        .await?;
        Ok(())
    }

    async fn comm_list(&mut self, session_id: &str) -> Result<Vec<AgentInfo>> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_request(Request::CommList {
            id,
            session_id: session_id.to_string(),
        })
        .await?;
        match self
                .read_until(Duration::from_secs(5), |event| {
                    matches!(event, ServerEvent::CommMembers { id: event_id, .. } if *event_id == id)
                })
                .await?
            {
                ServerEvent::CommMembers { members, .. } => Ok(members),
                other => anyhow::bail!("unexpected comm_list response: {other:?}"),
            }
    }

    async fn comm_status(
        &mut self,
        session_id: &str,
        target_session: &str,
    ) -> Result<AgentStatusSnapshot> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_request(Request::CommStatus {
            id,
            session_id: session_id.to_string(),
            target_session: target_session.to_string(),
        })
        .await?;
        match self
                .read_until(Duration::from_secs(5), |event| {
                    matches!(event, ServerEvent::CommStatusResponse { id: event_id, .. } if *event_id == id)
                })
                .await?
            {
                ServerEvent::CommStatusResponse { snapshot, .. } => Ok(snapshot),
                other => anyhow::bail!("unexpected comm_status response: {other:?}"),
            }
    }

    /// Wait for the next `Message` notification and return its scope
    /// ("dm", "channel", or "broadcast"). Other events are skipped.
    async fn next_message_notification(&mut self, timeout: Duration) -> Result<Option<String>> {
        match self
            .read_until(timeout, |event| {
                matches!(
                    event,
                    ServerEvent::Notification {
                        notification_type: NotificationType::Message { .. },
                        ..
                    }
                )
            })
            .await?
        {
            ServerEvent::Notification {
                notification_type: NotificationType::Message { scope, .. },
                ..
            } => Ok(scope),
            other => anyhow::bail!("unexpected notification response: {other:?}"),
        }
    }
}

async fn wait_for_server_socket(
    path: &Path,
    server_task: &mut tokio::task::JoinHandle<Result<()>>,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if server_task.is_finished() {
            let result = server_task.await?;
            return Err(anyhow::anyhow!(
                "server exited before socket became ready: {:?}",
                result
            ));
        }
        match Stream::connect(path).await {
            Ok(stream) => {
                drop(stream);
                return Ok(());
            }
            Err(err) => {
                if tokio::time::Instant::now() >= deadline {
                    return Err(err.into());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
}

fn test_ctx(session_id: &str, working_dir: &Path) -> ToolContext {
    ToolContext {
        session_id: session_id.to_string(),
        message_id: "msg-1".to_string(),
        tool_call_id: "call-1".to_string(),
        working_dir: Some(working_dir.to_path_buf()),
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: ToolExecutionMode::Direct,
    }
}

async fn spawn_attached_peer(
    tool: &CommunicateTool,
    requester_session: &str,
    working_dir: &Path,
    socket_path: &Path,
) -> Result<(RawClient, String)> {
    let spawn_output = tool
        .execute(
            json!({
                "action": "spawn",
                "label": "communicate e2e peer",
                "model": "inherit"
            }),
            test_ctx(requester_session, working_dir),
        )
        .await?;
    let spawned_session = spawn_output
        .output
        .strip_prefix("Spawned new agent: ")
        .ok_or_else(|| anyhow::anyhow!("spawn output omitted session id: {}", spawn_output.output))?
        .trim()
        .to_string();
    anyhow::ensure!(
        spawned_session != requester_session,
        "spawned member must remain distinct from requester"
    );

    let mut peer = RawClient::connect(socket_path).await?;
    peer.subscribe_to(working_dir, Some(&spawned_session))
        .await?;
    let attached_session = peer.session_id().await?;
    anyhow::ensure!(
        attached_session == spawned_session,
        "targeted subscribe attached {attached_session} instead of {spawned_session}"
    );
    Ok((peer, spawned_session))
}

async fn wait_for_member_status(
    client: &mut RawClient,
    requester_session: &str,
    target_session: &str,
    expected_status: &str,
) -> Result<Vec<AgentInfo>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let members = client.comm_list(requester_session).await?;
        if members
            .iter()
            .find(|member| member.session_id == target_session)
            .and_then(|member| member.status.as_deref())
            == Some(expected_status)
        {
            return Ok(members);
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for member {} to reach status {}",
                target_session,
                expected_status
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn wait_for_member_presence(
    client: &mut RawClient,
    requester_session: &str,
    target_session: &str,
) -> Result<Vec<AgentInfo>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let members = client.comm_list(requester_session).await?;
        if members
            .iter()
            .any(|member| member.session_id == target_session)
        {
            return Ok(members);
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for member {} to appear", target_session);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[test]
fn default_await_members_targets_include_ready() {
    assert_eq!(
        default_await_target_statuses(),
        vec!["ready", "completed", "stopped", "failed", "crashed"]
    );
}

fn credential_failed_worker(session_id: &str, detail: &str, age_secs: u64) -> AgentInfo {
    AgentInfo {
        session_id: session_id.to_string(),
        status: Some("failed".to_string()),
        detail: Some(detail.to_string()),
        role: Some("agent".to_string()),
        is_headless: Some(true),
        report_back_to_session_id: Some("coord".to_string()),
        status_age_secs: Some(age_secs),
        provider_name: Some("anthropic".to_string()),
        ..Default::default()
    }
}

#[test]
fn credential_failure_wave_detected_for_recent_auth_failed_workers() {
    // The observed incident: every dispatched worker died within seconds with
    // an Anthropic 401 (expired OAuth + revoked refresh token) and nothing
    // completed. That must classify as a wave, not as N independent failures.
    let members = vec![
        AgentInfo {
            session_id: "coord".to_string(),
            status: Some("running".to_string()),
            role: Some("coordinator".to_string()),
            ..Default::default()
        },
        credential_failed_worker("w1", "Anthropic API error (401 Unauthorized)", 2),
        credential_failed_worker("w2", "Anthropic API error (401 Unauthorized)", 3),
        credential_failed_worker("w3", "invalid_grant: refresh token invalid", 5),
    ];
    let wave = super::detect_credential_failure_wave(&members, "coord", 0, 60)
        .expect("three recent credential failures with zero completions is a wave");
    assert_eq!(wave.session_ids, vec!["w1", "w2", "w3"]);
    assert_eq!(wave.sample_detail, "Anthropic API error (401 Unauthorized)");
    assert_eq!(wave.provider.as_deref(), Some("anthropic"));

    let message = super::format_credential_failure_wave_error(&wave, 60);
    assert!(message.contains("paused dispatching"));
    assert!(message.contains("3 worker(s)"));
    assert!(message.contains("401 Unauthorized"));
    assert!(message.contains("`jcode login --provider claude`"));
}

#[test]
fn credential_failure_wave_requires_at_least_two_workers() {
    let members = vec![credential_failed_worker(
        "w1",
        "Anthropic API error (401 Unauthorized)",
        2,
    )];
    assert_eq!(
        super::detect_credential_failure_wave(&members, "coord", 0, 60),
        None,
        "one bad worker is not a wave"
    );
}

#[test]
fn credential_failure_wave_not_detected_once_anything_completed() {
    // Completions prove the credential works (or worked); later auth failures
    // are then per-worker problems, not a route-wide outage to halt over.
    let members = vec![
        credential_failed_worker("w1", "Anthropic API error (401 Unauthorized)", 2),
        credential_failed_worker("w2", "Anthropic API error (401 Unauthorized)", 3),
    ];
    assert_eq!(
        super::detect_credential_failure_wave(&members, "coord", 1, 60),
        None
    );
}

#[test]
fn credential_failure_wave_ignores_stale_and_non_credential_failures() {
    let members = vec![
        // Stale: failed long before this window (e.g. a previous, already
        // diagnosed wave; the user has since re-authenticated and retried).
        credential_failed_worker("old", "Anthropic API error (401 Unauthorized)", 3600),
        // Unknown age must not count either.
        AgentInfo {
            status_age_secs: None,
            ..credential_failed_worker("ageless", "401 Unauthorized", 0)
        },
        // Non-credential failure.
        credential_failed_worker("crashed", "worker panicked: index out of bounds", 2),
        // Only one recent credential failure remains: below the wave minimum.
        credential_failed_worker("w1", "Anthropic API error (401 Unauthorized)", 2),
    ];
    assert_eq!(
        super::detect_credential_failure_wave(&members, "coord", 0, 60),
        None
    );
}

#[test]
fn credential_failure_wave_ignores_foreign_members() {
    // A foreign, client-attached session that failed with an auth error is not
    // one of run_plan's workers; it must not trip the breaker.
    let foreign = AgentInfo {
        is_headless: Some(false),
        report_back_to_session_id: None,
        ..credential_failed_worker("foreign", "401 Unauthorized", 2)
    };
    let members = vec![
        foreign,
        credential_failed_worker("w1", "401 Unauthorized", 2),
    ];
    assert_eq!(
        super::detect_credential_failure_wave(&members, "coord", 0, 60),
        None
    );
}

#[test]
fn credential_login_fix_hint_maps_provider_names() {
    assert_eq!(
        super::credential_login_fix_hint(Some("anthropic")),
        "`jcode login --provider claude`"
    );
    assert_eq!(
        super::credential_login_fix_hint(Some("OpenAI")),
        "`jcode login --provider openai`"
    );
    assert_eq!(
        super::credential_login_fix_hint(Some("copilot")),
        "`jcode login --provider copilot`"
    );
    assert_eq!(
        super::credential_login_fix_hint(None),
        "`jcode login --provider <provider>`"
    );
}

#[test]
fn run_plan_terminal_summary_includes_recorded_failure_reasons() {
    let mut failed_reasons = std::collections::BTreeMap::new();
    failed_reasons.insert(
        "c".to_string(),
        "task failed: Anthropic API error (401 Unauthorized)".to_string(),
    );
    let summary = crate::protocol::PlanGraphStatus {
        swarm_id: Some("swarm-a".to_string()),
        version: 1,
        item_count: 2,
        ready_ids: Vec::new(),
        blocked_ids: Vec::new(),
        active_ids: Vec::new(),
        completed_ids: vec!["a".to_string()],
        failed_ids: vec!["c".to_string()],
        failed_reasons,
        cycle_ids: Vec::new(),
        unresolved_dependency_ids: Vec::new(),
        next_ready_ids: Vec::new(),
        newly_ready_ids: Vec::new(),
        low_confidence_ids: Vec::new(),
        mode: "light".to_string(),
        seeded_count: 0,
        grown_count: 0,
    };
    let output = super::format_run_plan_terminal_summary(3, &summary, 2);
    assert!(output.contains("Failed nodes: c"));
    assert!(
        output.contains("c: task failed: Anthropic API error (401 Unauthorized)"),
        "terminal summary must carry the recorded failure reason:\n{output}"
    );

    let plan_status = format_plan_status(&summary).output;
    assert!(
        plan_status.contains("c: task failed: Anthropic API error (401 Unauthorized)"),
        "plan_status must display the recorded failure reason:\n{plan_status}"
    );
}

include!("communicate_tests/input_format.rs");
include!("communicate_tests/end_to_end.rs");
include!("communicate_tests/assignment.rs");
