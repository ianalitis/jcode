/// What one turn produced.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TurnResult {
    /// None for natural completion. Failures still return Err and are also
    /// delivered to on_event as TurnStopped before the legacy Error event.
    pub stop_reason: Option<jcode_harness_api::TurnStopReason>,
    pub stop_message: Option<String>,
    /// All assistant text in the turn, including tool narration.
    pub text: String,
    /// Last completed assistant message, or aggregate text on older bridges.
    pub final_text: String,
    /// Framed messages. Empty when connected to an older, unframed bridge.
    pub messages: Vec<AssistantTextMessage>,
    pub reasoning: String,
    pub tool_calls: Vec<ToolCall>,
    /// Usage from the latest provider call in this turn, not a sum of calls.
    pub usage: Option<Usage>,
}

/// One assistant text message, excluding interleaved reasoning.
#[derive(Debug, Clone, PartialEq)]
pub struct AssistantTextMessage {
    /// Stream-local correlator, not a persisted history message id.
    pub message_id: Option<String>,
    pub text: String,
}

#[derive(Default)]
struct TextCollector {
    parts: Vec<(AssistantTextMessage, bool)>,
}

impl TextCollector {
    fn index(&self, id: &Option<String>) -> Option<usize> {
        self.parts
            .iter()
            .rposition(|(part, done)| &part.message_id == id && (id.is_some() || !done))
    }

    fn message(&mut self, id: Option<String>) -> &mut (AssistantTextMessage, bool) {
        let index = self.index(&id).unwrap_or_else(|| {
            self.parts.push((
                AssistantTextMessage {
                    message_id: id,
                    text: String::new(),
                },
                false,
            ));
            self.parts.len() - 1
        });
        &mut self.parts[index]
    }

    fn finish(self, result: &mut TurnResult) {
        result.text = self
            .parts
            .iter()
            .map(|(part, _)| part.text.as_str())
            .collect();
        result.messages = self
            .parts
            .into_iter()
            .filter_map(|(part, done)| (done && !part.text.is_empty()).then_some(part))
            .collect();
        result.final_text = result
            .messages
            .last()
            .map(|part| part.text.clone())
            .unwrap_or_else(|| result.text.clone());
    }
}
