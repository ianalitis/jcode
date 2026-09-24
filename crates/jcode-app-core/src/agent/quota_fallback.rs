//! Declared model ladder for a spent subscription quota window.
//!
//! When a route answers 429 with a daily/weekly/monthly window exhausted, no
//! retry can help and every model behind the same subscription shares the
//! window. Without this the turn simply stops, which for a heavily automated
//! harness means the whole pipeline stalls until the window resets.
//!
//! The ladder is operator-declared (`[provider].quota_fallback`), tried in
//! order, announced to the user on every step, and never applied to workers
//! running under a frozen spawn envelope: those carry a fixed treatment whose
//! receipt must stay attributable, so a quota stop there ends the attempt.

use super::Agent;
use crate::logging;
use crate::protocol::ServerEvent;
use tokio::sync::mpsc;

/// The model half of a `profile:model` ladder entry.
fn entry_model(entry: &str) -> &str {
    // Model ids may themselves contain ':' (OpenRouter `:batch` variants), so
    // only strip a leading profile prefix, which never contains '/'.
    match entry.split_once(':') {
        Some((prefix, rest)) if !prefix.contains('/') && !rest.is_empty() => rest,
        _ => entry,
    }
}

/// The next ladder entry to try, skipping the model that just failed and any
/// entry already attempted in this turn. Pure so it can be tested exhaustively.
pub(super) fn next_quota_fallback<'a>(
    ladder: &'a [String],
    current_model: &str,
    tried: &[String],
) -> Option<&'a str> {
    ladder
        .iter()
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .find(|entry| {
            !entry_model(entry).eq_ignore_ascii_case(current_model.trim())
                && !tried.iter().any(|t| t.eq_ignore_ascii_case(entry))
        })
}

impl Agent {
    /// On a spent quota window, switch to the next declared ladder entry and
    /// return a one-line notice for the caller to surface. `None` means the
    /// error is not a quota stop, the ladder is empty or exhausted, or this
    /// agent runs a frozen treatment; the caller then fails as before.
    pub(super) fn try_quota_window_fallback(
        &mut self,
        error: &str,
        tried: &mut Vec<String>,
    ) -> Option<String> {
        if self.spawn_execution_envelope.is_some()
            || !jcode_provider_core::is_exhausted_quota_window_error(error)
        {
            return None;
        }
        let ladder = crate::config::config().provider.quota_fallback.clone();
        let from = self.provider.model();
        loop {
            let entry = next_quota_fallback(&ladder, &from, tried)?.to_string();
            tried.push(entry.clone());
            match self.set_model_from_auth(&entry) {
                Ok(()) => {
                    let notice = format!(
                        "⚡ Quota window spent on {from}; switched to {entry} (declared quota_fallback ladder) and retrying."
                    );
                    logging::warn(&notice);
                    return Some(notice);
                }
                Err(err) => {
                    logging::warn(&format!(
                        "quota_fallback: cannot select '{entry}': {err}; trying the next entry"
                    ));
                }
            }
        }
    }
}

impl Agent {
    /// Streaming-loop wrapper: on a ladder switch, resync the client's model
    /// chip and context limit and show the user why the model changed.
    pub(super) fn quota_fallback_mpsc(
        &mut self,
        error: &str,
        tried: &mut Vec<String>,
        event_tx: &mpsc::UnboundedSender<ServerEvent>,
    ) -> bool {
        let Some(notice) = self.try_quota_window_fallback(error, tried) else {
            return false;
        };
        let _ = event_tx.send(ServerEvent::ModelChanged {
            id: 0,
            model: self.provider.model(),
            provider_name: Some(self.provider.display_name()),
            resolved_credential: None,
            error: None,
        });
        let _ = event_tx.send(ServerEvent::StatusDetail { detail: notice });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder(entries: &[&str]) -> Vec<String> {
        entries.iter().map(|e| e.to_string()).collect()
    }

    #[test]
    fn picks_first_entry_that_is_not_the_failed_model() {
        let l = ladder(&["opencode-go:deepseek-v4.1-flash", "mlx-serve:local-27b"]);
        assert_eq!(
            next_quota_fallback(&l, "deepseek-v4.1-flash", &[]),
            Some("mlx-serve:local-27b")
        );
    }

    #[test]
    fn walks_the_ladder_without_repeating_tried_entries() {
        let l = ladder(&["opencode-go:glm-5.3-flash", "mlx-serve:local-27b"]);
        let tried = vec!["opencode-go:glm-5.3-flash".to_string()];
        assert_eq!(
            next_quota_fallback(&l, "deepseek-v4.1-flash", &tried),
            Some("mlx-serve:local-27b")
        );
        let tried = ladder(&["opencode-go:glm-5.3-flash", "mlx-serve:local-27b"]);
        assert_eq!(next_quota_fallback(&l, "deepseek-v4.1-flash", &tried), None);
    }

    #[test]
    fn model_ids_with_slashes_and_colons_are_not_mistaken_for_profiles() {
        assert_eq!(
            entry_model("openrouter:deepseek/x:batch"),
            "deepseek/x:batch"
        );
        assert_eq!(entry_model("deepseek/x:batch"), "deepseek/x:batch");
        assert_eq!(entry_model("gpt-6-luna"), "gpt-6-luna");
    }

    #[test]
    fn empty_ladder_never_falls_back() {
        assert_eq!(next_quota_fallback(&[], "anything", &[]), None);
        assert_eq!(next_quota_fallback(&ladder(&["  "]), "anything", &[]), None);
    }
}
