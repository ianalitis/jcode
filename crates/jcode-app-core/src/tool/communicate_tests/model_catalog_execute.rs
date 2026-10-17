#![cfg(unix)]

use super::{EnvGuard, test_ctx};
use crate::protocol::{Request, ServerEvent};
use crate::tool::Tool;
use crate::tool::communicate::CommunicateTool;
use anyhow::{Context, Result};
use jcode_provider_core::ModelRoute;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

fn route(model: &str, provider: &str, api_method: &str, detail: &str) -> ModelRoute {
    ModelRoute {
        model: model.to_string(),
        provider: provider.to_string(),
        api_method: api_method.to_string(),
        available: true,
        detail: detail.to_string(),
        cheapness: None,
        usage: None,
    }
}

async fn execute_catalog(input: Value, response_routes: Vec<ModelRoute>) -> Result<String> {
    let _env_lock = crate::storage::lock_test_env();
    let root = tempfile::tempdir()?;
    let runtime = root.path().join("runtime");
    std::fs::create_dir_all(&runtime)?;
    let socket = runtime.join("catalog.sock");
    let listener = UnixListener::bind(&socket)?;
    let _home = EnvGuard::set("JCODE_HOME", root.path());
    let _runtime = EnvGuard::set("JCODE_RUNTIME_DIR", &runtime);
    let _socket = EnvGuard::set("JCODE_SOCKET", &socket);
    let _tmp = EnvGuard::set("TMPDIR", root.path());
    let working = root.path().join("work");
    std::fs::create_dir(&working)?;
    let session_id = "catalog-execute-session";
    let responder = async {
        let (stream, _) = listener.accept().await.context("accept catalog client")?;
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;
        let value: Value = serde_json::from_str(&request_line)?;
        let request: Request = serde_json::from_value(value.clone())?;
        let Request::CommListModels {
            id,
            session_id: received_session,
        } = request
        else {
            anyhow::bail!("unexpected request: {value}");
        };
        anyhow::ensure!(id == 1, "unexpected request id: {id}");
        anyhow::ensure!(
            received_session == session_id,
            "unexpected session: {received_session}"
        );
        anyhow::ensure!(value.get("provider").is_none());
        anyhow::ensure!(value.get("query").is_none());
        anyhow::ensure!(value.get("limit").is_none());
        let event = ServerEvent::CommListModelsResponse {
            id,
            current_model: Some("coordinator-model".into()),
            configured_swarm_model: Some("pinned-model".into()),
            model_routes: response_routes,
        };
        let mut stream = reader.into_inner();
        stream
            .write_all(format!("{}\n", serde_json::to_string(&event)?).as_bytes())
            .await?;
        stream.flush().await?;
        Result::<()>::Ok(())
    };
    let tool = CommunicateTool {
        description: String::new(),
    };
    let execute = tool.execute(input, test_ctx(session_id, &working));
    let ((), output) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::try_join!(responder, async { Ok::<_, anyhow::Error>(execute.await?) })
    })
    .await
    .context("catalog execute/transport timed out")??;
    Ok(output.output)
}

#[tokio::test]
async fn execute_catalog_actions_filter_limit_and_transport_only_action_request() -> Result<()> {
    let routes = vec![
        route("decoy-needle-oauth", "OpenAI", "openai-oauth", "oauth"),
        route("decoy-api", "OpenAI", "openai-api", "decoy"),
        route("needle-first", "OpenAI", "openai-api", "first"),
        route("needle-second", "OpenAI", "openai-api", "second"),
        route("needle-third", "OpenAI", "openai-api", "third"),
    ];
    let canonical = execute_catalog(
        json!({"action": "list_models", "provider": " OPENAI-API ", "query": " NeEdLe ", "limit": 2}),
        routes.clone(),
    ).await?;
    let alias = execute_catalog(
        json!({"action": "models", "provider": " OPENAI-API ", "query": " NeEdLe ", "limit": 2}),
        routes,
    )
    .await?;
    assert_eq!(canonical, alias);
    let rows: Vec<_> = canonical
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();
    assert_eq!(
        rows,
        [
            "- needle-first via OpenAI [openai-api] (first)",
            "- needle-second via OpenAI [openai-api] (second)",
        ]
    );
    for expected in [
        "Current coordinator model: coordinator-model\n",
        "Configured agents.swarm_model default: pinned-model\n",
        "- needle-first via OpenAI [openai-api] (first)\n",
        "- needle-second via OpenAI [openai-api] (second)\n",
        "Catalog counts: incoming=5, matched=3, shown=2, omitted=1, truncated=true, reason=limit\n",
    ] {
        assert!(
            canonical.contains(expected),
            "missing {expected:?} in {canonical}"
        );
    }
    assert!(!canonical.contains("decoy-"));
    assert!(!canonical.contains("needle-third"));
    Ok(())
}

#[tokio::test]
async fn execute_catalog_omits_whole_oversized_multibyte_row_with_zero_fit_explanation()
-> Result<()> {
    let row_detail = "é".repeat(9_000);
    let output = execute_catalog(
        json!({"action": "list_models"}),
        vec![route("oversized", "OpenAI", "openai-api", &row_detail)],
    )
    .await?;
    assert!(output.len() <= 16_384);
    assert!(output.contains("Catalog counts: incoming=1, matched=1, shown=0, omitted=1, truncated=true, reason=output_bytes\n"));
    assert!(output.contains(
        "Zero routes displayed due to the display budget: the first matching row cannot fit whole."
    ));
    assert!(!output.lines().any(|line| line.starts_with("- ")));
    assert!(!output.contains('é'));
    assert!(!output.contains("- oversized via"));
    Ok(())
}
