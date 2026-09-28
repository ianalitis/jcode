use crate::{ApiContentBlock, ApiMessage};

pub(super) fn merge_consecutive(messages: Vec<ApiMessage>) -> Vec<ApiMessage> {
    // Anthropic requires strictly alternating user/assistant messages.
    let pre_merge_count = messages.len();
    let mut merged: Vec<ApiMessage> = Vec::new();
    for msg in messages {
        if let Some(last) = merged.last_mut()
            && last.role == msg.role
        {
            last.content.extend(msg.content);
            continue;
        }
        merged.push(msg);
    }

    if merged.len() != pre_merge_count {
        jcode_logging::info(&format!(
            "[anthropic] Merged {} consecutive same-role messages",
            pre_merge_count - merged.len()
        ));
    }

    // Sibling text moved out of tool results can interleave with results from
    // parallel tool searches. Anthropic requires all results before other
    // blocks in the merged user turn. Partition stably to preserve their order.
    for msg in merged.iter_mut().filter(|m| m.role == "user") {
        let first_non_result = msg
            .content
            .iter()
            .position(|b| !matches!(b, ApiContentBlock::ToolResult { .. }));
        let needs_reorder = first_non_result.is_some_and(|start| {
            msg.content[start..]
                .iter()
                .any(|b| matches!(b, ApiContentBlock::ToolResult { .. }))
        });
        if needs_reorder {
            let (results, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut msg.content)
                .into_iter()
                .partition(|b| matches!(b, ApiContentBlock::ToolResult { .. }));
            msg.content = results;
            msg.content.extend(rest);
        }
    }
    merged
}
