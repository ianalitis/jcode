pub fn openai_compatible_profile_static_models(profile: OpenAiCompatibleProfile) -> Vec<String> {
    let mut models = Vec::new();
    let mut push = |model: &str| {
        let model = model.trim();
        if !model.is_empty() && !models.iter().any(|existing| existing == model) {
            models.push(model.to_string());
        }
    };

    match profile.id {
        "opencode" => {
            push("minimax-m2.7");
            push("kimi-k2.5");
            push("glm-4.7");
            push("glm-5");
            push("claude-haiku-4-5");
            push("gpt-5.1-codex-max");
        }
        "opencode-go" => {
            push("minimax-m2.7");
            push("kimi-k2.5");
            push("glm-5");
            push("glm-5.1");
            push("deepseek-v4-flash");
            push("qwen3.5-plus");
        }
        "zai" => {
            push("glm-4.5");
            push("glm-4.7");
            push("glm-5");
            push("glm-5.1");
            push("glm-4.7-flash");
            push("glm-4.7-flashx");
        }
        "302ai" => {
            push("qwen3-235b-a22b-instruct-2507");
            push("glm-4.7");
            push("glm-5.1");
            push("MiniMax-M2");
            push("kimi-k2-0905-preview");
            push("claude-haiku-4-5");
        }
        "baseten" => {
            push("zai-org/GLM-4.7");
            push("zai-org/GLM-5");
            push("openai/gpt-oss-120b");
            push("moonshotai/Kimi-K2.6");
            push("moonshotai/Kimi-K2.5");
            push("deepseek-ai/DeepSeek-V4-Pro");
        }
        "conifer" => {
            push("claude-fable-5");
            push("claude-opus-5");
            push("claude-opus-4-8");
            push("claude-sonnet-5");
            push("claude-sonnet-4-6");
            push("claude-haiku-4-5");
            push("gpt-5.6-sol");
            push("gpt-5.6-terra");
            push("gpt-5.6-luna");
            push("gpt-5.5");
            push("gpt-5.4");
            push("gpt-5.4-mini");
            push("gpt-5.4-nano");
            push("gemini-3.1-pro");
            push("gemini-3.1-pro-preview");
            push("gemini-3.7-flash");
            push("gemini-3.6-flash");
            push("gemini-3.5-flash");
            push("gemini-3.5-flash-lite");
            push("gemini-3.1-flash-lite");
            push("gemini-3-flash-preview");
            push("grok-4.6");
            push("grok-4.5");
            push("grok-4.3");
            push("kimi-k3");
            push("kimi-k3-together");
            push("kimi-k2.7-code");
            push("kimi-k2.7-code-highspeed");
            push("kimi-k2.7-code-nebius");
            push("kimi-k2.6");
            push("kimi-k2.6-fireworks");
            push("deepseek-v4-pro");
            push("deepseek-v4-pro-together");
            push("deepseek-v4-flash");
            push("deepseek-v4-flash-0731");
            push("deepseek-v4-flash-vision");
            push("deepseek-v4-flash-deepinfra");
            push("deepseek-v4-flash-0731-deepinfra");
            push("deepseek-v3.2");
            push("deepseek-v3.1-sambanova");
            push("glm-5.3");
            push("glm-5.3-flash");
            push("glm-5.2");
            push("glm-5.2-deepinfra");
            push("glm-5.2-sail");
            push("glm-4.7");
            push("glm-4.7-flash");
            push("glm-4.7-deepinfra");
            push("qwen3.8-max");
            push("qwen3.8-2.4t");
            push("qwen3.8-27b");
            push("qwen3.8-flash");
            push("qwen3.7-max");
            push("qwen3-max-thinking");
            push("qwen3-coder-480b");
            push("qwen3-vl-235b");
            push("qwen3-next-80b");
            push("minimax-m3");
            push("minimax-m3-novita");
            push("minimax-m3-deepinfra");
            push("minimax-m3-gmicloud");
            push("minimax-m2.7");
            push("mimo-v2.5-pro");
            push("mimo-v2.5-pro-xiaomi");
            push("mimo-v2.5-pro-novita");
            push("mimo-v2.5");
            push("mimo-v2.5-xiaomi");
            push("mimo-v2.5-novita");
            push("seed-2.0-pro");
            push("seed-2.0-code");
            push("seed-2.0-mini");
            push("step-3.7-flash");
            push("step-3.7-flash-novita");
            push("hy3");
            push("hy3-tencent");
            push("hy3-novita");
            push("ling-3.0-flash");
            push("inkling");
            push("inkling-small");
            push("nemotron-3-ultra");
            push("nemotron-3-super-120b");
            push("nemotron-3.5-lightning");
            push("mistral-large-latest");
            push("mistral-medium-latest");
            push("mistral-small-latest");
            push("command-a-cohere");
            push("llama-4-maverick");
            push("llama-4-scout");
            push("llama-3.3-70b");
            push("gpt-oss-120b");
            push("gpt-oss-120b-cerebras");
            push("gpt-oss-120b-deepinfra");
            push("gpt-oss-20b");
            push("gemma-4-31b");
            push("gemma-3-27b");
        }
        "cortecs" => {
            push("minimax-m2.7");
            push("kimi-k2.5");
            push("glm-4.7");
            push("glm-5");
            push("claude-haiku-4-5");
            push("qwen3-235b-a22b-instruct-2507");
        }
        // Issue #79: DeepSeek's live model catalog is not always available during
        // TUI startup, but both models should still be selectable once the direct
        // provider is configured.
        "deepseek" => {
            push("deepseek-v4-flash");
            push("deepseek-v4-pro");
        }
        "comtegra" => {
            push("gpt-oss-120b");
            push("qwen35-122b");
            push("gte-qwen2-7b");
            push("glm-51-nvfp4");
        }
        "fpt" => {
            push("GLM-5.1");
            push("GLM-4.7");
            push("Llama-3.3-70B-Instruct");
        }
        "kimi" => {
            push("kimi-for-coding");
            push("kimi-k2.5");
            push("kimi-k2.6");
            push("kimi-k2-thinking");
            push("kimi-k2-thinking-turbo");
        }
        "firmware" => {
            push("kimi-k2.5");
            push("zai-glm-5-1");
            push("claude-haiku-4-5");
            push("claude-sonnet-4-6");
            push("grok-code-fast-1");
            push("gemini-2.5-flash");
        }
        "huggingface" => {
            push("Qwen/Qwen3-Coder-480B-A35B-Instruct");
            push("Qwen/Qwen3-Coder-Next");
            push("zai-org/GLM-4.7");
            push("zai-org/GLM-5.1");
            push("deepseek-ai/DeepSeek-V3.2");
            push("openai/gpt-oss-120b");
        }
        "moonshotai" => {
            push("kimi-k2.5");
            push("kimi-k2.6");
            push("kimi-k2-thinking");
            push("kimi-k2-thinking-turbo");
            push("kimi-k2-turbo-preview");
        }
        "nebius" => {
            push("openai/gpt-oss-120b");
            push("Qwen/Qwen3-235B-A22B-Instruct-2507");
            push("Qwen/Qwen3.5-397B-A17B");
            push("zai-org/GLM-5");
            push("meta-llama/Llama-3.3-70B-Instruct");
            push("NousResearch/Hermes-4-70B");
        }
        "scaleway" => {
            push("qwen3-coder-30b-a3b-instruct");
            push("qwen3-235b-a22b-instruct-2507");
            push("qwen3.5-397b-a17b");
            push("gpt-oss-120b");
            push("mistral-small-3.2-24b-instruct-2506");
            push("llama-3.3-70b-instruct");
        }
        "stackit" => {
            push("openai/gpt-oss-120b");
            push("Qwen/Qwen3-VL-235B-A22B-Instruct-FP8");
            push("cortecs/Llama-3.3-70B-Instruct-FP8-Dynamic");
            push("neuralmagic/Meta-Llama-3.1-8B-Instruct-FP8");
            push("google/gemma-3-27b-it");
        }
        "perplexity" => {
            push("sonar");
            push("sonar-pro");
            push("sonar-reasoning-pro");
            push("sonar-deep-research");
        }
        "deepinfra" => {
            push("moonshotai/Kimi-K2-Instruct");
            push("Qwen/Qwen3-Coder-480B-A35B-Instruct");
            push("Qwen/Qwen3-Coder-480B-A35B-Instruct-Turbo");
            push("zai-org/GLM-4.7");
            push("zai-org/GLM-5.1");
            push("meta-llama/Llama-3.1-70B-Instruct");
        }
        "fireworks" => {
            push("accounts/fireworks/routers/kimi-k2p5-turbo");
            push("accounts/fireworks/models/kimi-k2p5");
            push("accounts/fireworks/models/kimi-k2p6");
            push("accounts/fireworks/models/glm-4p7");
            push("accounts/fireworks/models/glm-5p1");
            push("accounts/fireworks/models/deepseek-v3p2");
        }
        "cerebras" => {
            push("gpt-oss-120b");
            push("zai-glm-4.7");
        }
        // Keep coding models selectable while Novita's live catalog refreshes.
        "novita" => {
            push("zai-org/glm-5.3");
            push("zai-org/glm-5.3-flash");
            push("moonshotai/kimi-k3");
            push("deepseek/deepseek-v4-pro-0813");
        }
        // Belvedir's router accepts `auto`, but does not expose `/models` at
        // its OpenAI-compatible inference base.
        "belvedir" => push("auto"),
        // Celeris serves exactly one model per base URL today, and `/models`
        // requires auth, so keep the documented id available pre-refresh.
        "celeris" => {
            push("celeris-1");
        }
        "xiaomi-mimo" => {
            push("mimo-v2.5");
            push("mimo-v2.5-pro");
            push("mimo-v2-pro");
            push("mimo-v2-flash");
            push("mimo-v2-omni");
        }
        // Meta's catalog is authenticated, so expose the documented Muse Spark
        // models immediately after login while the live refresh completes.
        "meta-muse" => {
            push("muse-spark-1.2");
            push("muse-spark-1.1");
        }
        // MiniMax's `/models` endpoint is authenticated and live, but post-login
        // model activation should not depend on the catalog refresh completing
        // before the picker/routes are rebuilt. Keep the documented text models
        // selectable immediately after saving a key.
        "minimax" => {
            push("MiniMax-M2.7");
            push("MiniMax-M2.7-highspeed");
            push("MiniMax-M2.5");
            push("MiniMax-M2.5-highspeed");
            push("MiniMax-M2.1");
            push("MiniMax-M2.1-highspeed");
            push("MiniMax-M2");
        }
        "alibaba-coding-plan" => {
            push("qwen3-coder-plus");
            push("qwen3.5-plus");
            push("qwen3-max-2026-01-23");
            push("qwen3-coder-next");
            push("glm-5");
            push("glm-4.7");
            push("kimi-k2.5");
            push("MiniMax-M2.5");
        }
        "gemini-api" => {
            push("gemini-2.5-flash");
            push("gemini-2.5-pro");
            push("gemini-2.0-flash");
            push("gemini-2.0-flash-lite");
        }
        _ => {}
    }

    models
}

pub fn openai_compatible_profile_model_supports_chat(_profile_id: &str, _model: &str) -> bool {
    true
}

pub fn openai_compatible_profile_static_context_limits(
    profile: OpenAiCompatibleProfile,
) -> HashMap<String, usize> {
    openai_compatible_profile_static_models(profile)
        .into_iter()
        .filter_map(|model| {
            openai_compatible_profile_context_limit(profile.id, &model).map(|limit| (model, limit))
        })
        .collect()
}

/// Conifer's own published `context_window` for the models its catalog serves.
///
/// Observed 2026-09-20 from `GET https://api.conifer.build/v1/catalog` (260
/// entries, sha256 `ebc246a5b882057afd0e7571a9adbda498c494ece16670a958b1f2155993f621`,
/// raw capture: `~/.jcode/scratch/b10-conifer-catalog-20260920/catalog.json`).
/// The provider documents that endpoint as the source of truth and defines
/// `context_window` as the *input* budget: a prompt above it is refused rather
/// than trimmed, so using it as the model's context limit is conservative.
///
/// Ids absent from that catalog stay unresolved: an undocumented host alias must
/// not inherit another route's window. `nemotron-3-ultra-together` is the case
/// this guards, and it is also excluded from the static model list above.
fn conifer_catalog_context_limit(model: &str) -> Option<usize> {
    match model {
            "grok-4.6" => Some(500000),
            "grok-4.5" => Some(500000),
            "grok-4.3" => Some(1000000),
            "seed-2.0-pro" => Some(256000),
            "seed-2.0-code" => Some(256000),
            "seed-2.0-mini" => Some(256000),
            "step-3.7-flash" => Some(262144),
            "step-3.7-flash-novita" => Some(262144),
            "hy3" => Some(262144),
            "hy3-tencent" => Some(262144),
            "hy3-novita" => Some(262144),
            "ling-3.0-flash" => Some(131072),
            "inkling" => Some(524288),
            "inkling-small" => Some(524288),
            "nemotron-3-ultra" => Some(262144),
            "nemotron-3-super-120b" => Some(262144),
            "nemotron-3.5-lightning" => Some(262144),
            "mistral-large-latest" => Some(256000),
            "mistral-medium-latest" => Some(256000),
            "mistral-small-latest" => Some(256000),
            "command-a-cohere" => Some(256000),
            "llama-4-maverick" => Some(1048576),
            "llama-4-scout" => Some(327680),
            "gemma-4-31b" => Some(128000),
        _ => None,
    }
}

pub fn openai_compatible_profile_context_limit(profile_id: &str, model: &str) -> Option<usize> {
    let profile_id = profile_id.trim().to_ascii_lowercase();
    let model = model.trim().to_ascii_lowercase();

    match profile_id.as_str() {
        // The selected upstream model may vary. Use Jcode's conservative
        // compatible-provider context budget for the Belvedir auto router.
        "belvedir" if model == "auto" => Some(128_000),
        // DeepSeek V4 direct API models advertise a 1M token context window. The
        // direct profile runs through the OpenRouter/OpenAI-compatible provider
        // implementation, whose live catalog can be unavailable during startup.
        "deepseek" if model.starts_with("deepseek-v4-") => Some(1_000_000),
        // Conifer serves its own published per-model windows; prefer them over
        // the family classifier, which cannot see host-specific values.
        "conifer" => conifer_catalog_context_limit(&model)
            .or_else(|| conifer_context_limit(&model))
            .or_else(|| jcode_provider_core::models::open_weight_family_context_limit(&model)),
        // Fall back to the shared open-weight family classifier. Many bundled
        // OpenAI-compatible gateways (Z.AI/GLM, Moonshot/Kimi, MiniMax, Qwen,
        // etc.) serve `/v1/models` entries without a `context_length`, so this
        // static table is the only reliable source before a live catalog (or an
        // explicit user `context_window` override) is available.
        _ => jcode_provider_core::models::open_weight_family_context_limit(&model),
    }
}
