use super::*;
use crate::tool::Registry;
use std::collections::HashSet;

struct JsonTool;

#[async_trait::async_trait]
impl Tool for JsonTool {
    fn name(&self) -> &str {
        "lookup"
    }

    fn description(&self) -> &str {
        "Return structured data for an id\nMore detail."
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"id": {"type": "number"}}})
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let id = input["id"].as_i64().unwrap_or_default();
        Ok(ToolOutput::new(
            json!({ "id": id, "square": id * id }).to_string(),
        ))
    }
}

struct TextTool;

#[async_trait::async_trait]
impl Tool for TextTool {
    fn name(&self) -> &str {
        "shout"
    }

    fn description(&self) -> &str {
        "Uppercase text"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput::new(
            input["text"].as_str().unwrap_or_default().to_uppercase(),
        ))
    }
}

fn ctx(session: &str) -> ToolContext {
    ToolContext {
        session_id: session.to_string(),
        message_id: "message".to_string(),
        tool_call_id: "codemode-call".to_string(),
        working_dir: None,
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: super::super::ToolExecutionMode::Direct,
    }
}

async fn registry() -> Registry {
    let registry = Registry::empty();
    let mut tools = registry.tools.write().await;
    tools.insert("lookup".to_string(), Arc::new(JsonTool));
    tools.insert("shout".to_string(), Arc::new(TextTool));
    tools.insert(
        "codemode".to_string(),
        Arc::new(CodemodeTool::new(registry.downgrade())),
    );
    drop(tools);
    registry
}

#[tokio::test(flavor = "multi_thread")]
async fn corrupt_store_is_reported_before_script_execution_and_not_overwritten() {
    let _lock = crate::storage::lock_test_env();
    let session = format!("codemode-corrupt-{}", uuid::Uuid::new_v4());
    let path = store_path(&session).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let corrupt = b"{\"private_fixture\":";
    std::fs::write(&path, corrupt).unwrap();
    let result = registry()
        .await
        .execute(
            "codemode",
            json!({"code": "store('replacement', true); return 'must not execute';"}),
            ctx(&session),
        )
        .await;
    let after = std::fs::read(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        result.is_err(),
        "a corrupt store must not silently become empty"
    );
    assert_eq!(after, corrupt);
}

struct BlockStoreTool(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl Tool for BlockStoreTool {
    fn name(&self) -> &str {
        "block_store"
    }

    fn description(&self) -> &str {
        "Create a fixture that makes persistence fail after one side effect"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let path = store_path(&ctx.session_id).unwrap();
        std::fs::create_dir_all(path)?;
        Ok(ToolOutput::new("side effect completed"))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_store_save_reports_loss_without_replaying_completed_tools() {
    let _lock = crate::storage::lock_test_env();
    let session = format!("codemode-save-failure-{}", uuid::Uuid::new_v4());
    let calls = Arc::new(AtomicUsize::new(0));
    let registry = registry().await;
    registry.tools.write().await.insert(
        "block_store".into(),
        Arc::new(BlockStoreTool(calls.clone())),
    );
    let out = registry
        .execute(
            "codemode",
            json!({"code": "const result = await tools.block_store({}); store('pending', 1); return result;"}),
            ctx(&session),
        )
        .await
        .expect("completed tool effects must not become a retryable tool error");
    std::fs::remove_dir(store_path(&session).unwrap()).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(out.output.contains("side effect completed"));
    assert!(
        out.output.contains("store was not persisted"),
        "{}",
        out.output
    );
    assert!(out.output.contains("Do not replay completed tool calls"));
}

#[test]
fn store_round_trip_absence_and_io_failures_are_distinct() {
    let _lock = crate::storage::lock_test_env();
    let session = format!("codemode-roundtrip-{}", uuid::Uuid::new_v4());
    let path = store_path(&session).unwrap();
    assert!(load_store(&session).unwrap().is_empty());
    let values = Map::from_iter([("kept".into(), json!([1, 2, 3]))]);
    save_store(&session, &values).unwrap();
    assert_eq!(load_store(&session).unwrap(), values);
    save_store(&session, &Map::new()).unwrap();
    save_store(&session, &Map::new()).unwrap();
    assert!(!path.exists());
    std::fs::create_dir(&path).unwrap();
    assert!(load_store(&session).is_err());
    assert!(save_store(&session, &Map::new()).is_err());
    std::fs::remove_dir(path).unwrap();
    assert!(store_path("").is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn script_composes_registry_tools() {
    let registry = registry().await;
    let out = registry
        .execute(
            "codemode",
            json!({
                "code": r#"
                    const rows = await Promise.all([1, 2, 3].map((id) => tools.lookup({ id })));
                    const loud = await tools.shout({ text: "done" });
                    console.log("rows", rows.length);
                    return { total: rows.reduce((s, r) => s + r.square, 0), loud };
                "#
            }),
            ctx("codemode-compose-test"),
        )
        .await
        .expect("codemode runs");
    assert!(out.output.contains("\"total\": 14"), "{}", out.output);
    assert!(out.output.contains("\"loud\": \"DONE\""), "{}", out.output);
    assert!(out.output.contains("console:\nrows 3"), "{}", out.output);
    assert!(out.output.contains("[4 tool calls"), "{}", out.output);
}

#[tokio::test(flavor = "multi_thread")]
async fn script_cannot_call_itself_or_batch() {
    let registry = registry().await;
    let out = registry
        .execute(
            "codemode",
            json!({ "code": "return listTools().map((t) => t.name);" }),
            ctx("codemode-exclusion-test"),
        )
        .await
        .expect("codemode runs");
    assert!(out.output.contains("lookup"), "{}", out.output);
    assert!(!out.output.contains("\"codemode\""), "{}", out.output);

    let err = registry
        .execute(
            "codemode",
            json!({ "code": "return await tools.codemode({ code: 'return 1' });" }),
            ctx("codemode-exclusion-test"),
        )
        .await;
    let text = match err {
        Ok(out) => out.output,
        Err(err) => err.to_string(),
    };
    assert!(text.contains("unknown tool 'codemode'"), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn session_policy_applies_inside_scripts() {
    let session = "codemode-policy-test";
    crate::tool::set_session_tool_policy(session, None, HashSet::from(["shout".to_string()]));
    let registry = registry().await;
    let out = registry
        .execute(
            "codemode",
            json!({ "code": "return [listTools().map((t) => t.name), 'shout' in tools];" }),
            ctx(session),
        )
        .await;
    crate::tool::clear_session_tool_policy(session);
    let out = out.expect("codemode runs");
    assert!(!out.output.contains("\"shout\""), "{}", out.output);
    assert!(out.output.contains("false"), "{}", out.output);
}

#[tokio::test(flavor = "multi_thread")]
async fn uncaught_errors_without_side_effects_are_tool_errors() {
    let registry = registry().await;
    let err = registry
        .execute(
            "codemode",
            json!({ "code": "throw new Error('broken script')" }),
            ctx("codemode-error-test"),
        )
        .await
        .expect_err("pure failure is a tool error");
    assert!(err.to_string().contains("broken script"), "{err}");
}

#[test]
fn report_renders_strings_raw_and_lists_store_keys() {
    let mut store = Map::new();
    store.insert("k".into(), json!(1));
    let out = jcode_codemode::RunOutput {
        result: Some(json!("plain text")),
        tool_calls: 1,
        store,
        ..Default::default()
    };
    let report = render_report(&out);
    assert!(report.starts_with("result:\nplain text\n"), "{report}");
    assert!(report.contains("[1 tool call, "), "{report}");
    assert!(report.contains("stored: k]"), "{report}");
}

#[tokio::test]
async fn codemode_agent_subcalls_recheck_policy_after_revocation() {
    let session = "codemode-agent-revocation-test";
    let registry = registry().await;
    let mut context = ctx(session);
    context.execution_mode = crate::tool::ToolExecutionMode::AgentTurn;
    let host = RegistryHost {
        registry,
        ctx: context,
        counter: AtomicUsize::new(0),
    };
    assert!(
        host.call_tool("lookup".into(), json!({"id": 2}))
            .await
            .is_err()
    );
    let policy = crate::tool::register_session_tool_policy(
        session,
        Some(HashSet::from([
            "codemode".to_string(),
            "lookup".to_string(),
        ])),
        HashSet::new(),
    );
    assert_eq!(
        host.call_tool("lookup".into(), json!({"id": 2}))
            .await
            .unwrap(),
        json!({"id": 2, "square": 4})
    );
    assert!(
        host.call_tool("shout".into(), json!({"text": "blocked"}))
            .await
            .is_err()
    );
    drop(policy);
    assert!(
        host.call_tool("lookup".into(), json!({"id": 2}))
            .await
            .is_err()
    );
}
