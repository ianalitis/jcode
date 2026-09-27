use crate::message::{Message, ToolDefinition};
use crate::protocol::ServerEvent;
use std::hash::{Hash, Hasher};

fn stable_hash_str(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn stable_hash_json<T: serde::Serialize + ?Sized>(value: &T) -> u64 {
    let encoded = serde_json::to_string(value).unwrap_or_default();
    stable_hash_str(&encoded)
}

fn stable_json_len<T: serde::Serialize + ?Sized>(value: &T) -> usize {
    serde_json::to_string(value)
        .map(|encoded| encoded.len())
        .unwrap_or_default()
}

pub(super) fn kv_cache_request_event(
    messages: &[Message],
    tools: &[ToolDefinition],
    system_static: &str,
    ephemeral_messages: &[Message],
) -> ServerEvent {
    let ephemeral_hash = if ephemeral_messages.is_empty() {
        None
    } else {
        Some(stable_hash_json(ephemeral_messages))
    };
    // One projection and serialization pass feeds both the aggregate hash and
    // the per-message hashes. Computing them separately projected every message
    // twice and serialized the transcript N + 2 times per provider request.
    let signature = crate::message::cache_signature(messages);
    ServerEvent::KvCacheRequest {
        system_static_hash: stable_hash_str(system_static),
        tools_hash: stable_hash_json(tools),
        messages_hash: signature.aggregate,
        message_hashes: signature.per_message,
        message_count: messages.len(),
        tool_count: tools.len(),
        system_static_chars: system_static.chars().count(),
        tools_json_chars: stable_json_len(tools),
        messages_json_chars: stable_json_len(messages),
        ephemeral_hash,
        ephemeral_chars: stable_json_len(ephemeral_messages),
        ephemeral_message_count: ephemeral_messages.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_cache_request_tracks_empty_inputs_without_ephemeral_hash() {
        let event = kv_cache_request_event(&[], &[], "system", &[]);
        let ServerEvent::KvCacheRequest {
            message_count,
            tool_count,
            ephemeral_hash,
            system_static_chars,
            ..
        } = event
        else {
            panic!("expected KV cache request");
        };
        assert_eq!(message_count, 0);
        assert_eq!(tool_count, 0);
        assert_eq!(system_static_chars, 6);
        assert_eq!(ephemeral_hash, None);
    }
}
