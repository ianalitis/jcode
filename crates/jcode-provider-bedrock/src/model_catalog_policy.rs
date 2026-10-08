use super::{
    BedrockModelInfo, BedrockProvider, DEFAULT_CONTEXT_LIMIT, DEFAULT_MAX_OUTPUT_TOKENS,
    RouteCheapnessEstimate, RouteCostConfidence, RouteCostSource,
};
use std::collections::HashMap;

impl BedrockProvider {
    pub(super) fn normalize_model_id(model: &str) -> String {
        let mut value = model.trim().to_string();
        if let Some((_, tail)) = value.rsplit_once('/') {
            value = tail.to_string();
        }
        for prefix in ["us.", "eu.", "apac.", "global."] {
            if let Some(stripped) = value.strip_prefix(prefix) {
                value = stripped.to_string();
                break;
            }
        }
        value
    }

    #[cfg(feature = "aws-sdk")]
    pub(super) fn foundation_model_id_from_arn(arn: &str) -> Option<String> {
        arn.rsplit_once("foundation-model/")
            .map(|(_, model)| model.trim())
            .filter(|model| !model.is_empty())
            .map(str::to_string)
    }

    pub(super) fn inference_profile_id_from_arn(arn: &str) -> Option<String> {
        arn.rsplit_once("inference-profile/")
            .map(|(_, profile)| profile.trim())
            .filter(|profile| !profile.is_empty())
            .map(str::to_string)
    }

    pub(super) fn foundation_model_id_from_profile_id(profile_id: &str) -> Option<String> {
        let id = profile_id.trim();
        let id = Self::inference_profile_id_from_arn(id).unwrap_or_else(|| id.to_string());
        for prefix in ["us.", "eu.", "apac.", "global."] {
            if let Some(model) = id.strip_prefix(prefix)
                && !model.is_empty()
            {
                return Some(model.to_string());
            }
        }
        None
    }

    pub(super) fn region_profile_prefix() -> Option<&'static str> {
        let region = Self::configured_region()?;
        if region.starts_with("us-") {
            Some("us.")
        } else if region.starts_with("eu-") {
            Some("eu.")
        } else if region.starts_with("ap-") {
            Some("apac.")
        } else {
            None
        }
    }

    pub(super) fn inference_profile_priority(profile_id: &str) -> u8 {
        let id = profile_id.trim().to_ascii_lowercase();
        if let Some(prefix) = Self::region_profile_prefix()
            && id.starts_with(prefix)
        {
            return 0;
        }
        if id.starts_with("us.") || id.starts_with("eu.") || id.starts_with("apac.") {
            1
        } else if id.starts_with("global.") {
            2
        } else {
            3
        }
    }

    pub(super) fn insert_preferred_profile_route(
        routes: &mut HashMap<String, String>,
        foundation_model: &str,
        profile_id: &str,
    ) {
        let foundation_model = foundation_model.trim();
        let profile_id = profile_id.trim();
        if foundation_model.is_empty() || profile_id.is_empty() {
            return;
        }
        let should_replace = routes
            .get(foundation_model)
            .map(|current| {
                Self::inference_profile_priority(profile_id)
                    < Self::inference_profile_priority(current)
            })
            .unwrap_or(true);
        if should_replace {
            routes.insert(foundation_model.to_string(), profile_id.to_string());
        }
    }

    pub(super) fn merge_profile_routes_from_profile_ids(
        routes: &mut HashMap<String, String>,
        profiles: impl IntoIterator<Item = impl AsRef<str>>,
    ) {
        for profile in profiles {
            let profile = profile.as_ref().trim();
            let Some(foundation_model) = Self::foundation_model_id_from_profile_id(profile) else {
                continue;
            };
            let profile_id =
                Self::inference_profile_id_from_arn(profile).unwrap_or_else(|| profile.to_string());
            Self::insert_preferred_profile_route(routes, &foundation_model, &profile_id);
        }
    }

    pub(super) fn profile_route_for_model(&self, model: &str) -> Option<String> {
        let model = model.trim();
        if model.is_empty() {
            return None;
        }

        if let Ok(routes) = self.inference_profile_routes.read()
            && let Some(route) = routes.get(model).cloned()
        {
            return Some(route);
        }

        if let Ok(profiles) = self.fetched_inference_profiles.read() {
            let mut derived = HashMap::new();
            Self::merge_profile_routes_from_profile_ids(&mut derived, profiles.iter());
            if let Some(route) = derived.get(model).cloned() {
                return Some(route);
            }
        }

        None
    }

    pub fn is_bedrock_model_id(model: &str) -> bool {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            return false;
        }
        if trimmed.starts_with("arn:aws:bedrock:") {
            return true;
        }

        let id = Self::normalize_model_id(trimmed).to_ascii_lowercase();
        id.starts_with("anthropic.")
            || id.starts_with("amazon.")
            || id.starts_with("cohere.")
            || id.starts_with("ai21.")
            || id.starts_with("meta.")
            || id.starts_with("mistral.")
            || id.starts_with("stability.")
            || id.starts_with("writer.")
            || id.starts_with("deepseek.")
            || id.starts_with("openai.")
            || id.starts_with("qwen.")
            || id.starts_with("moonshot.")
            || id.starts_with("moonshotai.")
            || id.starts_with("minimax.")
            || id.starts_with("zai.")
            || id.starts_with("google.")
            || id.starts_with("nvidia.")
    }

    pub(super) fn model_info(model: &str) -> BedrockModelInfo {
        let id = Self::normalize_model_id(model).to_ascii_lowercase();
        if id.contains("claude-opus-4") || id.contains("claude-sonnet-4") {
            BedrockModelInfo {
                context_tokens: 200_000,
                max_output_tokens: 64_000,
                supports_tools: true,
                supports_vision: true,
                supports_reasoning: true,
                pricing: Some((3_000_000, 15_000_000)),
            }
        } else if id.contains("claude-3-7-sonnet") || id.contains("claude-3-5-sonnet") {
            BedrockModelInfo {
                context_tokens: 200_000,
                max_output_tokens: 8_192,
                supports_tools: true,
                supports_vision: true,
                supports_reasoning: id.contains("3-7"),
                pricing: Some((3_000_000, 15_000_000)),
            }
        } else if id.contains("claude-3-5-haiku") || id.contains("claude-3-haiku") {
            BedrockModelInfo {
                context_tokens: 200_000,
                max_output_tokens: 8_192,
                supports_tools: true,
                supports_vision: true,
                supports_reasoning: false,
                pricing: Some((800_000, 4_000_000)),
            }
        } else if id.contains("amazon.nova-pro") {
            BedrockModelInfo {
                context_tokens: 300_000,
                max_output_tokens: 5_120,
                supports_tools: true,
                supports_vision: true,
                supports_reasoning: false,
                pricing: Some((800_000, 3_200_000)),
            }
        } else if id.contains("amazon.nova-2-lite") || id.contains("amazon.nova-lite") {
            BedrockModelInfo {
                context_tokens: 300_000,
                max_output_tokens: 5_120,
                supports_tools: true,
                supports_vision: true,
                supports_reasoning: false,
                pricing: Some((60_000, 240_000)),
            }
        } else if id.contains("amazon.nova-micro") {
            BedrockModelInfo {
                context_tokens: 128_000,
                max_output_tokens: 5_120,
                supports_tools: true,
                supports_vision: false,
                supports_reasoning: false,
                pricing: Some((35_000, 140_000)),
            }
        } else if id.starts_with("deepseek.") {
            BedrockModelInfo {
                context_tokens: 128_000,
                max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
                supports_tools: false,
                supports_vision: false,
                supports_reasoning: true,
                pricing: None,
            }
        } else if id.contains("llama3-1-405b") || id.starts_with("meta.") {
            BedrockModelInfo {
                context_tokens: 128_000,
                max_output_tokens: 4_096,
                supports_tools: false,
                supports_vision: false,
                supports_reasoning: false,
                pricing: Some((5_320_000, 16_000_000)),
            }
        } else if id.starts_with("mistral.") {
            BedrockModelInfo {
                context_tokens: 128_000,
                max_output_tokens: 8_192,
                supports_tools: false,
                supports_vision: false,
                supports_reasoning: false,
                pricing: Some((4_000_000, 12_000_000)),
            }
        } else if id.starts_with("openai.")
            || id.starts_with("qwen.")
            || id.starts_with("moonshot.")
            || id.starts_with("moonshotai.")
            || id.starts_with("minimax.")
            || id.starts_with("zai.")
            || id.starts_with("google.")
            || id.starts_with("nvidia.")
            || id.starts_with("writer.")
        {
            BedrockModelInfo {
                context_tokens: DEFAULT_CONTEXT_LIMIT,
                max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
                supports_tools: false,
                supports_vision: false,
                supports_reasoning: id.contains("thinking")
                    || id.contains("reason")
                    || id.contains("gpt-oss"),
                pricing: None,
            }
        } else {
            BedrockModelInfo {
                context_tokens: DEFAULT_CONTEXT_LIMIT,
                max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
                supports_tools: false,
                supports_vision: false,
                supports_reasoning: false,
                pricing: None,
            }
        }
    }

    pub(super) fn route_pricing(model: &str) -> Option<RouteCheapnessEstimate> {
        let info = Self::model_info(model);
        info.pricing.map(|(input, output)| {
            RouteCheapnessEstimate::metered(
                RouteCostSource::Heuristic,
                RouteCostConfidence::Medium,
                input,
                output,
                None,
                Some("AWS Bedrock public on-demand pricing heuristic; verify for your region/account".to_string()),
            )
        })
    }

    pub(super) fn known_models() -> Vec<&'static str> {
        vec![
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "anthropic.claude-3-5-haiku-20241022-v1:0",
            "anthropic.claude-3-7-sonnet-20250219-v1:0",
            "anthropic.claude-sonnet-4-20250514-v1:0",
            "anthropic.claude-opus-4-20250514-v1:0",
            "amazon.nova-pro-v1:0",
            "amazon.nova-lite-v1:0",
            "amazon.nova-micro-v1:0",
            "meta.llama3-1-405b-instruct-v1:0",
            "mistral.mistral-large-2407-v1:0",
        ]
    }
}
