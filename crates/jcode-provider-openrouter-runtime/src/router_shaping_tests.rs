// J3 of docs/plans/TOKEN_ECONOMY_PLAN.md: OpenRouter request shaping and
// served-model receipts for dynamic-router attempts.

use super::attempt_caller::{AttemptOutcome, LocalLedger, run_frozen_attempt};
use super::single_send_tests::{TestServer, fixture_request, response, synthetic_provider};
use super::*;
use bytes::Bytes;
use chrono::Utc;
use futures::StreamExt;
use jcode_attempt_types::{
    AttemptRecord, DataClass, Effort, LocalBudget, ReceiptError, RouteClass, RouterPolicy,
    validate_receipt_for_gate,
};
use jcode_provider_openrouter::stream::OpenRouterStream;
use serde_json::json;

fn events_from(body: &str) -> Vec<StreamEvent> {
    let chunks = vec![Ok::<Bytes, reqwest::Error>(Bytes::from(body.to_string()))];
    let mut stream = OpenRouterStream::new(
        futures::stream::iter(chunks),
        "openrouter/auto-beta".to_string(),
        Arc::new(Mutex::new(None)),
    );
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut out = Vec::new();
        while let Some(event) = stream.next().await {
            out.push(event.unwrap());
        }
        out
    })
}

const ROUTED_SSE: &str = concat!(
    "data: {\"id\":\"gen-1\",\"model\":\"xiaomi/mimo-v2.5\",\"provider\":\"Novita\",",
    "\"choices\":[{\"delta\":{\"content\":\"ok\"}}],",
    "\"openrouter_metadata\":{\"pipeline\":[{\"type\":\"router\",\"data\":{\"task_type\":\"code:general_impl\"}},{\"type\":\"provider\"}]}}\n\n",
    "data: {\"id\":\"gen-1\",\"model\":\"xiaomi/mimo-v2.5\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":266,\"completion_tokens\":120,\"cost\":0.0000534048}}\n\n",
    "data: [DONE]\n\n"
);

#[test]
fn stream_emits_served_model_once_with_cost_and_task_type() {
    let events = events_from(ROUTED_SSE);
    let served: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::ServedModel {
                model,
                micro_usd,
                task_type,
            } => Some((model.clone(), *micro_usd, task_type.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        served,
        vec![(
            "xiaomi/mimo-v2.5".to_string(),
            Some(53),
            Some("code:general_impl".to_string())
        )],
        "served model is emitted once, with the billed cost from the usage chunk"
    );
    assert!(events.iter().any(|e| matches!(
        e,
        StreamEvent::TokenUsage {
            input_tokens: Some(266),
            output_tokens: Some(120),
            ..
        }
    )));
}

#[test]
fn stream_cost_and_model_on_one_chunk_emit_together() {
    let body = concat!(
        "data: {\"model\":\"z-ai/glm-5\",\"choices\":[{\"delta\":{\"content\":\"x\"},\"finish_reason\":\"stop\"}],",
        "\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"cost\":0.00207}}\n\n",
        "data: [DONE]\n\n"
    );
    let events = events_from(body);
    assert!(events.iter().any(|e| matches!(
        e,
        StreamEvent::ServedModel {
            micro_usd: Some(2070),
            task_type: None,
            ..
        }
    )));
}

#[test]
fn stream_without_model_field_emits_no_served_model() {
    let events = events_from(
        "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, StreamEvent::ServedModel { .. }))
    );
}

#[test]
fn provider_routing_serializes_zdr_and_data_collection() {
    let routing = ProviderRouting {
        zdr: Some(true),
        data_collection: Some("deny".into()),
        ..ProviderRouting::default()
    };
    assert!(!routing.is_empty());
    assert!(ProviderRouting::default().is_empty());
}

fn router_attempt(model: &str, policy: Option<RouterPolicy>) -> jcode_attempt_types::FrozenAttempt {
    AttemptRecord {
        task_id: "t".into(),
        attempt_id: format!("t/n1/a-{}", uuid::Uuid::new_v4()),
        node_id: "n1".into(),
        provider: "openrouter".into(),
        model_exact: model.into(),
        endpoint: "loopback".into(),
        route_class: RouteClass::MeteredRemote,
        effort: Effort::Low,
        tool_allowlist: vec![],
        data_class: DataClass::Public,
        router: policy,
        deadline_secs: 5,
        budget: LocalBudget {
            max_input_bytes: 4096,
            max_output_bytes: 4096,
            max_micro_usd: 2_070,
            max_generations: 1,
        },
        prompt_hash: "p".repeat(64),
        policy_version: "test".into(),
    }
    .freeze(Utc::now())
    .unwrap()
}

fn exclusions() -> RouterPolicy {
    RouterPolicy {
        excluded_models: vec!["openai/*".into(), "anthropic/*".into()],
        cost_tier: Some("low".into()),
    }
}

fn run_router_attempt(
    body: &'static str,
    attempt: &jcode_attempt_types::FrozenAttempt,
) -> Result<super::attempt_caller::AttemptResult, super::attempt_caller::CallerError> {
    let server = TestServer::spawn(vec![response("200 OK", body)]);
    let provider = synthetic_provider(server.api_base.clone());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        *provider.model.write().await = attempt.record().model_exact.clone();
    });
    let messages = vec![Message::user("approved prompt")];
    let mut expected = fixture_request(&messages);
    expected["model"] = json!(attempt.record().model_exact);
    let ledger = LocalLedger::new(2_070);
    let result = rt.block_on(run_frozen_attempt(
        &provider,
        attempt,
        &ledger,
        expected,
        &server.destination,
        &messages,
        &[],
        "",
        None,
    ));
    assert_eq!(server.join(), 1);
    result
}

#[test]
fn router_attempt_receipt_names_served_model_and_task_type_and_settles_billed_cost() {
    let attempt = router_attempt("openrouter/auto-beta", Some(exclusions()));
    let r = run_router_attempt(ROUTED_SSE, &attempt).unwrap();
    assert_eq!(r.outcome, AttemptOutcome::Completed { text: "ok".into() });
    assert_eq!(r.receipt.binary_id, "openrouter:xiaomi/mimo-v2.5");
    assert_eq!(r.receipt.task_type.as_deref(), Some("code:general_impl"));
    assert_eq!(r.receipt.usage.as_ref().unwrap().micro_usd, Some(53));
    assert!(validate_receipt_for_gate(&r.receipt, &attempt).is_ok());
}

#[test]
fn router_attempt_serving_a_banned_family_is_rejected_at_the_receipt() {
    const LEAKED: &str = concat!(
        "data: {\"model\":\"openai/gpt-5.6-sol\",\"provider\":\"Azure\",\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}],",
        "\"usage\":{\"prompt_tokens\":24,\"completion_tokens\":65,\"cost\":0.00207}}\n\n",
        "data: [DONE]\n\n"
    );
    let attempt = router_attempt("openrouter/auto-beta", Some(exclusions()));
    let err = run_router_attempt(LEAKED, &attempt).unwrap_err();
    let expected = ReceiptError::ServedModelBanned {
        served: "openai/gpt-5.6-sol".into(),
    }
    .to_string();
    assert!(err.to_string().contains(&expected), "{err}");
}

#[test]
fn concrete_attempt_receipt_keeps_frozen_model_when_stream_names_none() {
    let attempt = router_attempt("approved/model", None);
    let r = run_router_attempt(
        "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        &attempt,
    )
    .unwrap();
    assert_eq!(r.receipt.binary_id, "openrouter:approved/model");
    assert_eq!(r.receipt.task_type, None);
}

#[test]
fn billed_cost_from_stream_settles_the_ledger_below_the_reservation() {
    const BILLED: &str = concat!(
        "data: {\"model\":\"z-ai/glm-5\",\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}],",
        "\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"cost\":0.000120}}\n\n",
        "data: [DONE]\n\n"
    );
    let attempt = router_attempt("openrouter/auto-beta", Some(exclusions()));
    let r = run_router_attempt(BILLED, &attempt).unwrap();
    assert_eq!(r.receipt.usage.as_ref().unwrap().micro_usd, Some(120));
    assert_eq!(r.receipt.binary_id, "openrouter:z-ai/glm-5");
}

#[test]
fn served_model_without_usage_chunk_is_still_emitted_before_message_end() {
    let events = events_from(
        "data: {\"model\":\"z-ai/glm-5\",\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
    );
    let served = events.iter().position(|e| {
        matches!(
            e,
            StreamEvent::ServedModel {
                micro_usd: None,
                ..
            }
        )
    });
    let end = events
        .iter()
        .position(|e| matches!(e, StreamEvent::MessageEnd { .. }));
    assert!(
        served.is_some() && end.is_some() && served < end,
        "{events:?}"
    );
}

fn capture_openrouter_request(routing: ProviderRouting) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut buf = vec![0u8; 16384];
        let n = stream.read(&mut buf).unwrap_or(0);
        let _ = tx.send(String::from_utf8_lossy(&buf[..n]).into_owned());
        let body = "data: [DONE]\n\n";
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        );
    });
    let provider = OpenRouterProvider {
        api_base: format!("http://{addr}/v1"),
        supports_provider_features: true,
        send_openrouter_headers: true,
        conversation_id: "conv-fixed-1".to_string(),
        provider_routing: Arc::new(RwLock::new(routing)),
        ..synthetic_provider(String::new())
    };
    let messages = vec![Message::user("hello")];
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut stream = provider.complete(&messages, &[], "", None).await.unwrap();
        while let Some(event) = stream.next().await {
            event.unwrap();
        }
    });
    rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap()
}

#[test]
fn openrouter_requests_carry_session_id_metadata_header_and_privacy_routing() {
    let request = capture_openrouter_request(ProviderRouting {
        order: Some(vec!["novita".into()]),
        allow_fallbacks: false,
        zdr: Some(true),
        data_collection: Some("deny".into()),
        ..ProviderRouting::default()
    });
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    assert!(!headers.to_ascii_lowercase().contains("http-referer:"));
    assert!(!headers.to_ascii_lowercase().contains("x-title:"));
    assert!(
        headers
            .to_ascii_lowercase()
            .contains("x-openrouter-metadata: enabled"),
        "{headers}"
    );
    let body: serde_json::Value = serde_json::from_str(body.trim()).unwrap();
    assert_eq!(body["session_id"], "conv-fixed-1");
    assert_eq!(body["provider"]["zdr"], true);
    assert_eq!(body["provider"]["data_collection"], "deny");
    assert_eq!(body["provider"]["allow_fallbacks"], false);
    assert_eq!(body["provider"]["order"], json!(["novita"]));
}
