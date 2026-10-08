use super::Provider;
use anyhow::Result;
use jcode_message_types::{ContentBlock, Message, Role, StreamEvent};

/// Drive `Provider::complete` for a single user prompt and collect the text and
/// provider-reported usage.
pub async fn collect_simple_completion<P: Provider + ?Sized>(
    provider: &P,
    prompt: &str,
    system: &str,
) -> Result<(String, SimpleCompletionUsage)> {
    use futures::StreamExt;

    let messages = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: prompt.to_string(),
            cache_control: None,
        }],
        timestamp: None,
        tool_duration_ms: None,
    }];

    let response = provider.complete(&messages, &[], system, None).await?;
    let mut result = String::new();
    let mut usage = SimpleCompletionUsage::default();
    tokio::pin!(response);

    while let Some(event) = response.next().await {
        match event {
            Ok(StreamEvent::TextDelta(text)) => result.push_str(&text),
            Ok(StreamEvent::TokenUsage {
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
            }) => usage.observe(
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
            ),
            Ok(_) => {}
            Err(err) => return Err(err),
        }
    }

    Ok((result, usage))
}

/// Provider-reported usage for a non-agent completion.
///
/// Streams may report usage more than once (for example a start event with
/// input tokens and a final event with output tokens). Each field keeps the
/// latest value reported, matching how the agent turn loop treats repeated
/// `TokenUsage` events (they are cumulative snapshots, not deltas).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SimpleCompletionUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

impl SimpleCompletionUsage {
    pub fn observe(
        &mut self,
        input: Option<u64>,
        output: Option<u64>,
        cache_read: Option<u64>,
        cache_creation: Option<u64>,
    ) {
        if input.is_some() {
            self.input_tokens = input;
        }
        if output.is_some() {
            self.output_tokens = output;
        }
        if cache_read.is_some() {
            self.cache_read_input_tokens = cache_read;
        }
        if cache_creation.is_some() {
            self.cache_creation_input_tokens = cache_creation;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.input_tokens.is_none()
            && self.output_tokens.is_none()
            && self.cache_read_input_tokens.is_none()
            && self.cache_creation_input_tokens.is_none()
    }
}
