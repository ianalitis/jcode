/// Whether the hosted `image_generation` tool can be attached for `model_id`.
///
/// The Responses backend only exposes `image_generation` to general
/// ChatGPT/GPT models. Codex models (ids containing `codex`) reject unknown
/// hosted tools, so they must not receive it. See issue #369.
/// Responses `additional_tools` / deferred tool loading requires gpt-5.4 or
/// newer. Unknown model families stay eager, which is always correct.
fn model_supports_additional_tools(model_id: &str) -> bool {
    let lower = model_id.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("gpt-") else {
        return false;
    };
    let version: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = version.split('.').filter(|p| !p.is_empty());
    let major: u32 = match parts.next().and_then(|p| p.parse().ok()) {
        Some(major) => major,
        None => return false,
    };
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    major > 5 || (major == 5 && minor >= 4)
}

fn model_supports_image_generation(model_id: &str) -> bool {
    !model_id.to_ascii_lowercase().contains("codex")
}

#[cfg(test)]
mod model_capability_tests {
    use super::*;

    #[test]
    fn deferred_tools_require_gpt_5_4_or_newer() {
        assert!(!model_supports_additional_tools("gpt-5.3-codex"));
        assert!(model_supports_additional_tools("gpt-5.4"));
        assert!(model_supports_additional_tools("GPT-6"));
        assert!(!model_supports_additional_tools("o3"));
    }

    #[test]
    fn codex_models_cannot_attach_hosted_image_generation() {
        assert!(!model_supports_image_generation("gpt-5.4-codex"));
        assert!(model_supports_image_generation("gpt-5.4"));
    }
}
