//! Parsing of OpenAI-compatible `/v1/models` catalog responses.
//!
//! Kept separate from the provider runtime so the shape-tolerance rules for the
//! many gateways jcode speaks to (vLLM, llama.cpp, Ollama, LM Studio, vendor
//! clouds) live in one small, testable place.

use anyhow::{Context, Result};
use jcode_provider_openrouter::{ModelInfo, ModelPricing};
use serde_json::Value;

pub(crate) fn parse_openai_compatible_models_response(raw_body: &str) -> Result<Vec<ModelInfo>> {
    let value: Value = serde_json::from_str(raw_body)?;
    let items = match &value {
        Value::Array(items) => items,
        Value::Object(object) => object
            .get("data")
            .or_else(|| object.get("models"))
            .and_then(Value::as_array)
            .context("missing model array")?,
        _ => anyhow::bail!("model catalog response must be an object or array"),
    };

    let mut models = Vec::new();
    for item in items {
        if let Some(model) = parse_model_info_value(item) {
            models.push(model);
        }
    }

    if models.is_empty() {
        anyhow::bail!("model catalog response did not contain any valid model objects");
    }

    Ok(models)
}

pub(crate) fn parse_model_info_value(value: &Value) -> Option<ModelInfo> {
    let object = value.as_object()?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| object.get("name").and_then(Value::as_str))?
        .to_string();
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| object.get("display_name").and_then(Value::as_str))
        .or_else(|| object.get("displayName").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();

    Some(ModelInfo {
        id,
        name,
        context_length: first_u64_field(
            object,
            &[
                "context_length",
                "context_window",
                "context_size",
                "contextLength",
                "max_context_length",
                "maxModelLength",
                "max_model_len",
                "trainingContextLength",
            ],
        )
        // llama.cpp's /v1/models reports the serving context only inside
        // `meta` (`n_ctx`, with `n_ctx_train` as the trained maximum). Without
        // this, local llama.cpp models fall back to the generic 200K default
        // and the context gauge overstates the real window (issue #447).
        .or_else(|| {
            object
                .get("meta")
                .and_then(Value::as_object)
                .and_then(|meta| first_u64_field(meta, &["n_ctx", "n_ctx_train"]))
        }),
        pricing: parse_model_pricing(object.get("pricing")),
        created: object.get("created").and_then(value_as_u64),
    })
}

pub(crate) fn first_u64_field(
    object: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<u64> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(value_as_u64))
}

pub(crate) fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
}

pub(crate) fn value_as_pricing_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

pub(crate) fn parse_model_pricing(value: Option<&Value>) -> ModelPricing {
    let Some(Value::Object(object)) = value else {
        return ModelPricing::default();
    };

    // `ModelPricing` holds USD per token (see
    // `openrouter_pricing_from_token_prices`, which scales by 1e12 to reach
    // micro-USD per million tokens). Some OpenAI-compatible gateways publish USD
    // per *million* tokens instead, under `*_usd_per_mtok` keys — the shape
    // Conifer's catalog and its pinned SDK use. Those must be scaled down by 1e6;
    // copying them verbatim would inflate every cost estimate by a million.
    // `list_*` variants are deliberately ignored: they are the undiscounted list
    // price, not what the request is billed at.
    ModelPricing {
        prompt: object
            .get("prompt")
            .or_else(|| object.get("input"))
            .and_then(value_as_pricing_string)
            .or_else(|| {
                object
                    .get("in_usd_per_mtok")
                    .and_then(usd_per_mtok_as_per_token_string)
            }),
        completion: object
            .get("completion")
            .or_else(|| object.get("output"))
            .and_then(value_as_pricing_string)
            .or_else(|| {
                object
                    .get("out_usd_per_mtok")
                    .and_then(usd_per_mtok_as_per_token_string)
            }),
        input_cache_read: object
            .get("input_cache_read")
            .or_else(|| object.get("cached_input"))
            .and_then(value_as_pricing_string)
            .or_else(|| {
                object
                    .get("cache_read_usd_per_mtok")
                    .and_then(usd_per_mtok_as_per_token_string)
            }),
        input_cache_write: object
            .get("input_cache_write")
            .and_then(value_as_pricing_string)
            .or_else(|| {
                object
                    .get("cache_write_usd_per_mtok")
                    .and_then(usd_per_mtok_as_per_token_string)
            }),
    }
}

/// Convert a USD-per-million-tokens price into the USD-per-token string
/// `ModelPricing` stores.
fn usd_per_mtok_as_per_token_string(value: &Value) -> Option<String> {
    let per_mtok = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(text) => match text.trim().parse::<f64>() {
            Ok(value) => value,
            Err(_) => return None,
        },
        _ => return None,
    };
    if !per_mtok.is_finite() || per_mtok < 0.0 {
        return None;
    }
    // Fixed-point with trailing zeros trimmed: `f64::to_string` would emit the
    // full shortest round-trip decimal (`0.00000010000000000000001`) for values
    // that are exact in decimal but not in binary. Fifteen decimals keeps prices
    // down to 1e-15 USD/token ($1e-9 per million) without exposing that tail.
    let mut text = format!("{:.15}", per_mtok / 1_000_000.0);
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novita_model_catalog_preserves_names_and_context_limits() {
        let models = parse_openai_compatible_models_response(
            r#"{"data":[{"id":"zai-org/glm-5.3","object":"model","owned_by":"novita","display_name":"GLM 5.3","context_size":1048576,"created":1787105676}]}"#,
        )
        .expect("Novita catalog response should parse");
        assert_eq!(models[0].id, "zai-org/glm-5.3");
        assert_eq!(models[0].name, "GLM 5.3");
        assert_eq!(models[0].context_length, Some(1_048_576));
        assert_eq!(models[0].created, Some(1_787_105_676));
    }

    #[test]
    fn conifer_per_million_pricing_is_scaled_to_usd_per_token() {
        // The observed Conifer catalog publishes USD per *million* tokens. The
        // stored contract is USD per token, so a verbatim copy would inflate
        // every estimate by 1e6.
        let models = parse_openai_compatible_models_response(
            r#"{"data":[{"id":"gpt-5.6-sol","context_window":1000000,"pricing":{"in_usd_per_mtok":0.1,"out_usd_per_mtok":0.2,"cache_read_usd_per_mtok":0.01,"cache_write_usd_per_mtok":0.5,"list_in_usd_per_mtok":0.3}}]}"#,
        )
        .expect("Conifer per-million catalog response should parse");

        let pricing = &models[0].pricing;
        assert_eq!(pricing.prompt.as_deref(), Some("0.0000001"));
        assert_eq!(pricing.completion.as_deref(), Some("0.0000002"));
        assert_eq!(pricing.input_cache_read.as_deref(), Some("0.00000001"));
        assert_eq!(pricing.input_cache_write.as_deref(), Some("0.0000005"));

        // End-to-end unit check: 0.1 USD per million input tokens must reach the
        // estimator as 100_000 micro-USD per million tokens.
        let estimate = jcode_provider_core::pricing::openrouter_pricing_from_token_prices(
            pricing.prompt.as_deref(),
            pricing.completion.as_deref(),
            pricing.input_cache_read.as_deref(),
            jcode_provider_core::RouteCostSource::PublicApiPricing,
            jcode_provider_core::RouteCostConfidence::Exact,
            None,
        )
        .expect("scaled prices must produce an estimate");
        assert_eq!(estimate.input_price_per_mtok_micros, Some(100_000));
        assert_eq!(estimate.output_price_per_mtok_micros, Some(200_000));
        assert_eq!(estimate.cache_read_price_per_mtok_micros, Some(10_000));
    }

    #[test]
    fn per_token_pricing_keys_win_over_per_million_keys() {
        let models = parse_openai_compatible_models_response(
            r#"{"data":[{"id":"mixed","pricing":{"input":"0.00008","in_usd_per_mtok":0.1}}]}"#,
        )
        .expect("mixed pricing shape should parse");

        assert_eq!(models[0].pricing.prompt.as_deref(), Some("0.00008"));
    }

    #[test]
    fn conifer_context_window_and_pricing_are_parsed() {
        let models = parse_openai_compatible_models_response(
            r#"{"data":[{"id":"gpt-5.6-sol","context_window":1000000,"pricing":{"input":"0.1","output":"0.2","cached_input":"0.01"}}]}"#,
        )
        .expect("Conifer catalog response should parse");

        assert_eq!(models[0].context_length, Some(1_000_000));
        assert_eq!(models[0].pricing.prompt.as_deref(), Some("0.1"));
        assert_eq!(models[0].pricing.completion.as_deref(), Some("0.2"));
        assert_eq!(models[0].pricing.input_cache_read.as_deref(), Some("0.01"));
    }
}
