use super::{anthropic_static_model_ids, normalize_model_id, openai_static_model_ids};
use std::collections::HashSet;

pub(crate) fn merge_openai_model_ids(dynamic_models: Vec<String>) -> Vec<String> {
    let mut models = openai_static_model_ids();
    let mut seen: HashSet<String> = models
        .iter()
        .map(|model| normalize_model_id(model))
        .collect();
    let mut extras = Vec::new();

    for model in dynamic_models {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized = normalize_model_id(trimmed);
        if normalized.is_empty() || !seen.insert(normalized) {
            continue;
        }

        extras.push(trimmed.to_string());
    }

    extras.sort();
    models.extend(extras);
    models
}

pub(crate) fn merge_anthropic_model_ids(dynamic_models: Vec<String>) -> Vec<String> {
    let mut models = anthropic_static_model_ids();
    let mut seen: HashSet<String> = models
        .iter()
        .map(|model| normalize_model_id(model))
        .collect();
    let mut extras = Vec::new();

    for model in dynamic_models {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized = normalize_model_id(trimmed);
        if normalized.is_empty() || !seen.insert(normalized) {
            continue;
        }

        extras.push(trimmed.to_string());
    }

    extras.sort();
    models.extend(extras);
    models
}
