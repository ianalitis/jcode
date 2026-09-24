//! Choosing the next Anthropic model after a "model not found" failure.

use super::AnthropicProvider;

/// Models that have been retired and must never be chosen as a fallback target
/// (the server 404s them, so picking one just loops). Matched as a substring of
/// the normalized id so dated variants are covered too. `claude-fable` was
/// briefly on this list while retired, but the model is live again.
const RETIRED_ANTHROPIC_MODEL_MARKERS: &[&str] = &["claude-mythos"];

pub(super) fn anthropic_model_is_retired(model: &str) -> bool {
    let normalized = AnthropicProvider::normalized_model_key(model);
    RETIRED_ANTHROPIC_MODEL_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

/// Quality rank for an Anthropic model id: lower is better. Uses the curated
/// flagship-first `ALL_CLAUDE_MODELS` order (Opus > Sonnet > Haiku > older), so
/// fallback never silently downgrades to a cheaper tier when a stronger model is
/// available. Unknown/uncurated ids sort after every curated one but before
/// retired models, which sort last.
pub(super) fn anthropic_model_quality_rank(model: &str) -> usize {
    if anthropic_model_is_retired(model) {
        return usize::MAX;
    }
    let normalized = jcode_provider_core::model_id::strip_date_suffix(
        &jcode_provider_core::model_id::canonical(model),
    )
    .to_string();
    jcode_provider_core::ALL_CLAUDE_MODELS
        .iter()
        .position(|candidate| {
            jcode_provider_core::model_id::strip_date_suffix(
                &jcode_provider_core::model_id::canonical(candidate),
            ) == normalized
        })
        // Curated models keep their position; unknown-but-not-retired models sort
        // just after the curated list so they only win when nothing curated is
        // available.
        .unwrap_or(jcode_provider_core::ALL_CLAUDE_MODELS.len())
}

/// Cut `text` at the first sentence boundary, keeping periods that sit between
/// two digits (version numbers such as "4.8").
fn hint_sentence(text: &str) -> &str {
    let bytes = text.as_bytes();
    for (idx, &byte) in bytes.iter().enumerate() {
        let boundary = match byte {
            b'!' | b'\n' => true,
            b'.' => {
                let prev_digit = idx > 0 && bytes[idx - 1].is_ascii_digit();
                let next_digit = bytes.get(idx + 1).is_some_and(u8::is_ascii_digit);
                !(prev_digit && next_digit)
            }
            _ => false,
        };
        if boundary {
            return &text[..idx];
        }
    }
    text
}

/// Parse a server-recommended replacement model from a 404 body, e.g.
/// "Claude Fable 5 is not available. Please use Opus 4.8." -> the catalog id
/// `claude-opus-4-8`. Returns the best matching known catalog id, if any.
/// `error_str` is expected to already be lowercased.
pub(super) fn anthropic_recommended_model_from_error(error_str: &str) -> Option<String> {
    // Look for the phrase after "please use" / "use " and try to match it against
    // the known catalog by collapsing it to a comparable token form. The server
    // phrases the recommendation in prose ("Opus 4.8"), so compare on the digits
    // and family word rather than exact ids.
    let hint = error_str
        .split("please use")
        .nth(1)
        .or_else(|| error_str.split("use ").nth(1))?;
    // Take up to the next sentence boundary. A period between two digits is a
    // version separator ("Opus 4.8."), not a boundary, or the hint would
    // truncate to "Opus 4" and tie every 4.x model.
    let hint = hint_sentence(hint).trim();
    if hint.is_empty() {
        return None;
    }
    // Reduce the hint to alphanumeric tokens (e.g. "opus", "4", "8").
    let hint_tokens: Vec<String> = hint
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect();
    if hint_tokens.is_empty() {
        return None;
    }
    // Score each known catalog model by how many hint tokens it contains.
    jcode_base::provider::known_anthropic_model_ids()
        .into_iter()
        .filter(|candidate| !anthropic_model_is_retired(candidate))
        .map(|candidate| {
            let key = AnthropicProvider::normalized_model_key(&candidate);
            // The catalog id uses hyphenated digits ("claude-opus-4-8"), so the
            // hint tokens ["opus","4","8"] should all appear.
            let mut candidate_tokens: Vec<&str> = key.split('-').collect();
            let score = hint_tokens
                .iter()
                .filter(|token| {
                    if let Some(index) = candidate_tokens
                        .iter()
                        .position(|part| *part == token.as_str())
                    {
                        candidate_tokens.remove(index);
                        true
                    } else {
                        false
                    }
                })
                .count();
            (candidate, score)
        })
        // Require at least the family word plus one version digit to match so we
        // do not pick an arbitrary model from a single shared token.
        .filter(|(_, score)| *score >= 2)
        .max_by_key(|(_, score)| *score)
        .map(|(candidate, _)| candidate)
}

/// Pick the next Anthropic model to try after a "model not found" failure.
///
/// Strategy (most authoritative first):
///   1. Honor any server "Please use X" recommendation parsed from the error.
///   2. Otherwise pick the highest-quality untried model from the curated
///      flagship-first catalog, skipping retired families so we never downgrade
///      to a cheaper tier (e.g. Haiku) while a stronger model is available.
///
/// Returns `None` once every viable candidate is exhausted so the caller can
/// surface the original error.
pub(super) fn anthropic_fallback_model(tried: &[String], error_str: &str) -> Option<String> {
    let already_tried = |candidate: &str| {
        tried.iter().any(|model| {
            AnthropicProvider::normalized_model_key(model)
                == AnthropicProvider::normalized_model_key(candidate)
        })
    };

    // 1. Server recommendation wins when it points at an untried, non-retired
    //    model.
    if let Some(recommended) = anthropic_recommended_model_from_error(error_str)
        && !already_tried(&recommended)
        && !anthropic_model_is_retired(&recommended)
    {
        return Some(recommended);
    }

    // 2. Best available by curated quality order, skipping retired and tried.
    jcode_base::provider::known_anthropic_model_ids()
        .into_iter()
        .filter(|candidate| !already_tried(candidate) && !anthropic_model_is_retired(candidate))
        .min_by_key(|candidate| anthropic_model_quality_rank(candidate))
}
