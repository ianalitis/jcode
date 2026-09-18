//! Pure reducer for Pi's `--mode rpc` JSONL event stream.
//!
//! See `@earendil-works/pi-coding-agent` `docs/rpc.md`. This module does no
//! I/O: it turns one protocol line at a time into accumulated assistant text,
//! usage and settlement state. Keeping it pure makes the framing and field
//! handling testable without spawning a process.

use jcode_attempt_types::Usage;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Accumulated result of a Pi RPC run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PiProtocol {
    /// Text of the last assistant `message_end` (authoritative).
    pub assistant_text: String,
    /// Concatenated `text_delta` chunks (used only if no `message_end` text).
    pub streamed_text: String,
    /// The run reached `agent_settled`.
    pub settled: bool,
    /// First error event or failed response seen.
    pub error: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Provider-reported cost for the run, in micro-USD.
    pub cost_usd_micros: Option<u64>,
    /// A `get_session_stats` response was folded in.
    pub stats_seen: bool,
}

impl PiProtocol {
    /// Fold one JSONL line. A non-JSON line is a protocol error, because pi
    /// promises strict JSONL on stdout.
    pub fn ingest_line(&mut self, line: &str) -> Result<(), String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }
        let value: Value =
            serde_json::from_str(line).map_err(|e| format!("bad pi rpc line: {e}"))?;
        match value.get("type").and_then(Value::as_str).unwrap_or("") {
            "message_update" => {
                if let Some(event) = value.get("assistantMessageEvent")
                    && event.get("type").and_then(Value::as_str) == Some("text_delta")
                    && let Some(delta) = event.get("delta").and_then(Value::as_str)
                {
                    self.streamed_text.push_str(delta);
                }
                self.capture_usage(value.get("usage"));
            }
            "message_end" => {
                if let Some(message) = value.get("message")
                    && message.get("role").and_then(Value::as_str) == Some("assistant")
                {
                    if let Some(text) = extract_text(message.get("content")) {
                        self.assistant_text = text;
                    }
                    self.capture_usage(message.get("usage"));
                }
            }
            "agent_settled" => self.settled = true,
            "error" | "extension_error" => {
                self.error = Some(
                    value
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown pi error")
                        .to_string(),
                );
            }
            "response"
                if value.get("command").and_then(Value::as_str) == Some("get_session_stats") =>
            {
                self.stats_seen = true;
                if value.get("success").and_then(Value::as_bool) == Some(false) {
                    self.error = Some(
                        value
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("get_session_stats failed")
                            .to_string(),
                    );
                } else if let Some(data) = value.get("data") {
                    self.capture_stats(data);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The text to cite in a receipt: authoritative `message_end`, else the
    /// streamed deltas joined.
    pub fn final_text(&self) -> String {
        if self.assistant_text.is_empty() {
            self.streamed_text.clone()
        } else {
            self.assistant_text.clone()
        }
    }

    pub fn usage(&self) -> Usage {
        Usage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            micro_usd: self.cost_usd_micros,
        }
    }

    fn capture_usage(&mut self, usage: Option<&Value>) {
        let Some(usage) = usage else {
            return;
        };
        if let Some(input) = usage.get("input").and_then(Value::as_u64) {
            self.input_tokens = input;
        }
        if let Some(output) = usage.get("output").and_then(Value::as_u64) {
            self.output_tokens = output;
        }
        if let Some(cost) = usage
            .get("cost")
            .and_then(|cost| cost.get("total"))
            .and_then(Value::as_f64)
        {
            self.cost_usd_micros = Some(usd_to_micros(cost));
        }
    }

    fn capture_stats(&mut self, data: &Value) {
        if let Some(tokens) = data.get("tokens") {
            if let Some(input) = tokens.get("input").and_then(Value::as_u64) {
                self.input_tokens = input;
            }
            if let Some(output) = tokens.get("output").and_then(Value::as_u64) {
                self.output_tokens = output;
            }
        }
        if let Some(cost) = data.get("cost").and_then(Value::as_f64) {
            self.cost_usd_micros = Some(usd_to_micros(cost));
        }
    }
}

fn usd_to_micros(cost_usd: f64) -> u64 {
    (cost_usd.max(0.0) * 1_000_000.0).round() as u64
}

fn extract_text(content: Option<&Value>) -> Option<String> {
    match content? {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let mut out = String::new();
            for item in items {
                if item.get("type").and_then(Value::as_str) == Some("text")
                    && let Some(text) = item.get("text").and_then(Value::as_str)
                {
                    out.push_str(text);
                }
            }
            Some(out)
        }
        _ => None,
    }
}
