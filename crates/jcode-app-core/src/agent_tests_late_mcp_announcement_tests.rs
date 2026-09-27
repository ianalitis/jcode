fn transcript_texts(agent: &Agent) -> Vec<String> {
    agent
        .session
        .messages
        .iter()
        .flat_map(|message| message.content.iter())
        .filter_map(|block| match block {
            ContentBlock::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn late_mcp_tools_are_announced_once_in_the_transcript() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Deferred;
    let before = agent.tool_definitions().await;
    agent
        .registry
        .register(
            "mcp__late__tool".into(),
            Arc::new(FakeMcpTool { name: "tool".into() }) as Arc<dyn crate::tool::Tool>,
        )
        .await;

    agent.announce_late_mcp_tools().await;
    let texts = transcript_texts(&agent);
    let announcements: Vec<_> = texts
        .iter()
        .filter(|text| text.contains("New MCP tools are available."))
        .collect();
    assert_eq!(announcements.len(), 1);
    let text = announcements[0];
    assert!(text.contains("mcp__late__tool"));
    assert!(text.contains("server: late"));
    assert!(text.contains("tool: tool"));
    assert!(text.contains("input_schema: {"), "{text}");
    assert!(text.contains("mcp_call"));
    agent.announce_late_mcp_tools().await;
    assert_eq!(
        transcript_texts(&agent)
            .iter()
            .filter(|text| text.contains("New MCP tools are available."))
            .count(),
        1
    );
    assert_eq!(
        serde_json::to_string(&before).unwrap(),
        serde_json::to_string(&agent.tool_definitions().await).unwrap()
    );
}

#[tokio::test]
async fn late_mcp_announcement_skips_referenced_native_and_eager() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    registry
        .register(
            "mcp__srv__known".into(),
            Arc::new(FakeMcpTool { name: "known".into() }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Deferred;
    agent.add_message(
        Role::User,
        vec![
            ContentBlock::ToolResult {
                tool_use_id: "call_1".into(),
                content: "Connected".into(),
                is_error: None,
            },
            ContentBlock::ToolReference {
                tool_use_id: "call_1".into(),
                tool_name: "mcp__srv__known".into(),
            },
        ],
    );
    agent.announce_late_mcp_tools().await;
    assert!(!transcript_texts(&agent)
        .iter()
        .any(|text| text.contains("New MCP tools are available.")));

    for mode in [
        crate::config::McpToolsMode::Eager,
        crate::config::McpToolsMode::Auto,
    ] {
        let mut agent = if mode == crate::config::McpToolsMode::Eager {
            let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
            let registry = Registry::new(provider.clone()).await;
            let mut agent = Agent::new(provider, registry);
            agent.mcp_tools_mode = mode;
            agent
        } else {
            native_deferred_agent(mode).await
        };
        agent
            .registry
            .register(
                "mcp__x__y".into(),
                Arc::new(FakeMcpTool { name: "y".into() }) as Arc<dyn crate::tool::Tool>,
            )
            .await;
        agent.announce_late_mcp_tools().await;
        assert!(!transcript_texts(&agent)
            .iter()
            .any(|text| text.contains("New MCP tools are available.")), "{mode:?}");
    }
}

struct IdentifiedFakeMcpTool {
    server: String,
    raw: String,
}

#[async_trait]
impl crate::tool::Tool for IdentifiedFakeMcpTool {
    fn name(&self) -> &str { &self.raw }
    fn mcp_identity(&self) -> Option<(&str, &str)> { Some((&self.server, &self.raw)) }
    fn description(&self) -> &str { "fake identified mcp tool" }
    fn parameters_schema(&self) -> serde_json::Value { serde_json::json!({"type": "object"}) }
    async fn execute(
        &self,
        _input: serde_json::Value,
        _ctx: crate::tool::ToolContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok(ToolOutput::new("ok"))
    }
}

#[tokio::test]
async fn late_mcp_announcement_uses_original_names_for_sanitized_aliases() {
    let _guard = crate::storage::lock_test_env();
    let provider: Arc<dyn Provider> = Arc::new(NativeAutoCompactionProvider);
    let registry = Registry::new(provider.clone()).await;
    register_fake_deferred_mcp_surface(&registry).await;
    let mut agent = Agent::new(provider, registry);
    agent.mcp_tools_mode = crate::config::McpToolsMode::Deferred;
    let _ = agent.tool_definitions().await;
    let alias = crate::mcp::dispatch_name("yc", "hiring.create_job");
    assert_eq!(alias, "mcp__yc__hiring_create_job");
    agent
        .registry
        .register(
            alias.clone(),
            Arc::new(IdentifiedFakeMcpTool {
                server: "yc".into(),
                raw: "hiring.create_job".into(),
            }) as Arc<dyn crate::tool::Tool>,
        )
        .await;
    agent.announce_late_mcp_tools().await;
    let text = transcript_texts(&agent)
        .into_iter()
        .find(|text| text.contains("New MCP tools are available."))
        .expect("announcement");
    assert!(text.contains(&alias), "{text}");
    assert!(text.contains("server: yc  tool: hiring.create_job"), "{text}");
}
