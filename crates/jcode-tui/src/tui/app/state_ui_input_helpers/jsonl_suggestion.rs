pub(super) fn jsonl_suggestion_role(value: &serde_json::Value) -> Option<&str> {
    value
        .get("message")
        .and_then(|message| message.get("role"))
        .or_else(|| value.get("role"))
        .or_else(|| value.get("payload").and_then(|payload| payload.get("role")))
        .or_else(|| value.get("type"))
        .and_then(|role| role.as_str())
}

pub(super) fn jsonl_suggestion_text(value: &serde_json::Value) -> Option<String> {
    let content = value
        .get("message")
        .and_then(|message| message.get("content"))
        .or_else(|| value.get("lastPrompt"))
        .or_else(|| value.get("content"))
        .or_else(|| {
            value
                .get("payload")
                .and_then(|payload| payload.get("content"))
        })?;
    if let Some(text) = content
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_string());
    }
    let text = content
        .as_array()?
        .iter()
        .filter_map(|block| {
            block
                .get("text")
                .or_else(|| block.get("input_text"))
                .or_else(|| block.get("output_text"))
                .or_else(|| block.get("content"))
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|text| !text.is_empty())
        })
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}

pub(super) fn compact_suggestion_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        return compact;
    }
    let mut truncated = compact
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}
