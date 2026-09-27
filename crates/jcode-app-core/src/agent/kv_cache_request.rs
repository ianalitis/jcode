use crate::message::{Message, ToolDefinition};
use crate::protocol::ServerEvent;
use std::hash::{Hash, Hasher};

impl super::Agent {
    fn should_track_client_cache(&self) -> bool {
        match std::env::var("JCODE_TRACK_CLIENT_CACHE") {
            Ok(value) => {
                let value = value.trim();
                !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
            }
            Err(_) => false,
        }
    }

    pub(super) fn record_client_cache_request(&mut self, messages: &[Message]) {
        if !self.should_track_client_cache() {
            return;
        }

        let fast_snapshot =
            if !self.provider.uses_jcode_compaction() && self.session.compaction.is_none() {
                let previous_count = self.cache_tracker.previous_message_count();
                let prefix_hashes = self.session.provider_message_prefix_hashes();
                let current_count = prefix_hashes.len();
                let current_full_hash = prefix_hashes.last().copied();
                let prefix_hash_at_previous_count =
                    if previous_count == 0 || previous_count > current_count {
                        None
                    } else {
                        Some(prefix_hashes[previous_count - 1])
                    };
                Some((
                    current_count,
                    prefix_hash_at_previous_count,
                    current_full_hash,
                ))
            } else {
                None
            };

        let violation =
            if let Some((current_count, prefix_hash_at_previous_count, current_full_hash)) =
                fast_snapshot
            {
                self.cache_tracker.record_prefix_hash_snapshot(
                    current_count,
                    prefix_hash_at_previous_count,
                    current_full_hash,
                )
            } else {
                self.cache_tracker.record_request(messages)
            };

        if let Some(violation) = violation {
            crate::logging::warn(&format!(
                "CLIENT_CACHE_VIOLATION: {} | turn={} messages={}",
                violation.reason, violation.turn, violation.message_count
            ));
        }
    }
}

fn stable_hash_str(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn stable_hash_json<T: serde::Serialize + ?Sized>(value: &T) -> u64 {
    match serde_json::to_string(value) {
        Ok(encoded) => stable_hash_str(&encoded),
        Err(error) => {
            crate::logging::warn(&format!("KV cache hash serialization failed: {error}"));
            stable_hash_str(std::any::type_name::<T>())
        }
    }
}

fn stable_json_len<T: serde::Serialize + ?Sized>(value: &T) -> usize {
    match serde_json::to_string(value) {
        Ok(encoded) => encoded.len(),
        Err(error) => {
            crate::logging::warn(&format!("KV cache length serialization failed: {error}"));
            0
        }
    }
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

    struct FailingSerialization;

    impl serde::Serialize for FailingSerialization {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("synthetic serialization failure"))
        }
    }

    #[test]
    fn serialization_failure_is_not_reported_as_empty_json_hash() {
        let failure = FailingSerialization;
        assert_ne!(stable_hash_json(&failure), stable_hash_str(""));
        assert_eq!(stable_json_len(&failure), 0);
    }

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
