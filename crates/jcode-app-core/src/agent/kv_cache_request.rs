use crate::message::{Message, ToolDefinition};
use crate::protocol::ServerEvent;
use std::hash::{Hash, Hasher};

impl super::Agent {
    /// Provider identity used for cache retention lookups. Generic OpenAI is
    /// only refined when the credential mode is explicitly pinned.
    fn kv_cache_provider_identity(&self) -> String {
        let name = self.provider.name().to_string();
        if !name.eq_ignore_ascii_case("openai") {
            return name;
        }
        match self.provider.active_explicit_credential() {
            Some(jcode_provider_core::ResolvedCredential::ApiKey) => "openai-api".into(),
            Some(jcode_provider_core::ResolvedCredential::Oauth) => "openai-oauth".into(),
            None => name,
        }
    }

    pub(super) fn begin_kv_cache_monitor_request(&mut self, event: &ServerEvent, model: &str) {
        let ServerEvent::KvCacheRequest {
            system_static_hash,
            tools_hash,
            messages_hash,
            message_hashes,
            message_count,
            tool_count,
            ..
        } = event
        else {
            return;
        };
        let provider = self.kv_cache_provider_identity();
        let route = crate::kv_cache_monitor::RequestRoute {
            cache_ttl_secs: crate::provider::cache_ttl_for_provider_model(&provider, Some(model)),
            ttl_is_estimate: crate::provider::cache_ttl_is_estimate(&provider),
            provider,
            model: model.to_string(),
            upstream_provider: self.last_upstream_provider.clone(),
        };
        let signature = crate::kv_cache_monitor::RequestSignature {
            system_static_hash: *system_static_hash,
            tools_hash: *tools_hash,
            tool_count: *tool_count,
            messages_hash: *messages_hash,
            message_hashes: message_hashes.clone(),
            message_count: *message_count,
        };
        self.kv_cache_monitor.begin_request(route, signature);
    }

    /// Classify the completed request's usage. Returns the event to send when
    /// the request missed the KV cache.
    pub(super) fn finish_kv_cache_monitor_request(
        &mut self,
        input: u64,
        cache_read: Option<u64>,
        cache_creation: Option<u64>,
    ) -> Option<ServerEvent> {
        let effective = self.effective_context_tokens_from_usage(input, cache_read, cache_creation);
        let miss = self
            .kv_cache_monitor
            .finish_request(effective, cache_read)?;
        crate::logging::warn(&format!(
            "KV_CACHE_MISS session={} reason={} harness_caused={} missed={} expected={} read={} documented={:?}",
            self.session.id,
            miss.reason.id(),
            miss.reason.harness_caused(),
            miss.missed_tokens,
            miss.expected_tokens,
            miss.read_tokens,
            miss.documented_cause,
        ));
        Some(ServerEvent::KvCacheMiss {
            reason: miss.reason.id().to_string(),
            harness_caused: miss.reason.harness_caused(),
            missed_tokens: miss.missed_tokens,
            expected_tokens: miss.expected_tokens,
            read_tokens: miss.read_tokens,
            message: miss.message(),
            documented_cause: miss.documented_cause,
        })
    }

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
