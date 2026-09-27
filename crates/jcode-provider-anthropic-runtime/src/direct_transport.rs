use super::API_URL;
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub(super) fn direct_api_url() -> String {
    let base = std::env::var("JCODE_ANTHROPIC_API_BASE")
        .ok()
        .or_else(|| std::env::var("ANTHROPIC_BASE_URL").ok())
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty());
    match base {
        Some(base) if base.ends_with("/messages") => base,
        Some(base) => format!("{base}/messages"),
        None => API_URL.to_string(),
    }
}

pub(super) fn configured_direct_headers() -> Result<HeaderMap> {
    let Some(raw) = std::env::var("JCODE_ANTHROPIC_HEADERS")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(HeaderMap::new());
    };
    let headers: std::collections::BTreeMap<String, String> = serde_json::from_str(&raw)
        .context("JCODE_ANTHROPIC_HEADERS must be a JSON object of string values")?;
    let mut result = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .with_context(|| format!("invalid Anthropic-compatible header name '{name}'"))?;
        let value = HeaderValue::from_str(&value)
            .with_context(|| format!("invalid value for Anthropic-compatible header '{name}'"))?;
        result.insert(name, value);
    }
    Ok(result)
}

pub(super) fn direct_auth_mode() -> String {
    std::env::var("JCODE_ANTHROPIC_AUTH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if std::env::var("ANTHROPIC_AUTH_TOKEN")
                .ok()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "bearer".to_string()
            } else {
                "header".to_string()
            }
        })
        .trim()
        .to_ascii_lowercase()
}
