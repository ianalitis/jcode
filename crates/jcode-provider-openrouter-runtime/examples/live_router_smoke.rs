//! Live smoke for J3: does a real OpenRouter Auto Router response carry the
//! served model, billed cost and task_type through the SSE parser?
//! Needs OPENROUTER_API_KEY. One bounded request, low cost tier.
//!
//! cargo run -p jcode-provider-openrouter-runtime --example live_router_smoke

use futures::StreamExt;
use jcode_message_types::{Message, StreamEvent};
use jcode_provider_core::Provider;
use jcode_provider_openrouter::ProviderRouting;
use jcode_provider_openrouter_runtime::OpenRouterProvider;

fn main() {
    let model = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "openrouter/auto-beta".to_string());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let provider = OpenRouterProvider::new_openrouter_api_key_runtime().expect("api key");
        provider.set_model(&model).expect("model");
        provider
            .set_provider_routing(ProviderRouting {
                zdr: Some(true),
                data_collection: Some("deny".into()),
                ..ProviderRouting::default()
            })
            .await;
        let messages = vec![Message::user(
            "Write a Rust fn that returns the max of a slice of i32, no prose.",
        )];
        let mut stream = provider
            .complete(&messages, &[], "", None)
            .await
            .expect("request");
        let mut text = String::new();
        while let Some(event) = stream.next().await {
            match event.expect("event") {
                StreamEvent::TextDelta(t) => text.push_str(&t),
                StreamEvent::ServedModel {
                    model,
                    micro_usd,
                    task_type,
                } => println!("served_model={model} micro_usd={micro_usd:?} task_type={task_type:?}"),
                StreamEvent::UpstreamProvider { provider } => println!("upstream_provider={provider}"),
                StreamEvent::TokenUsage {
                    input_tokens,
                    output_tokens,
                    cache_read_input_tokens,
                    ..
                } => println!(
                    "usage in={input_tokens:?} out={output_tokens:?} cached={cache_read_input_tokens:?}"
                ),
                _ => {}
            }
        }
        println!("text_bytes={}", text.len());
    });
}
