//! Tool-local catalog presentation. This does not bound daemon allocation or transfer.

use jcode_provider_core::{
    AuthRoute, ModelRoute, RuntimeKey, model_route_provider_labels_match,
    model_route_provider_matches_key,
};

const MAX_OUTPUT_BYTES: usize = 16 * 1024;
const EMPTY: &str = "No model routes reported.\n";
const NO_MATCH: &str = "No model routes match these filters. Clear or broaden provider/query.\n";
const NO_FIT: &str = "Zero routes displayed due to the display budget: the first matching row cannot fit whole. Narrow provider/query to select other rows.\n";
const FOOTER: &str = "\nOmit model to use the configured default, or pass inherit to use the coordinator.\nPass a bare model or route-pinned value to override the default, and effort (none|minimal|low|medium|high|xhigh|max) to set reasoning effort.\nNarrow with provider (exact family/route/profile) and/or query (literal model/provider/api_method substring).\nPresentation only: daemon allocation and transfer are not bounded.\n";

pub(super) struct CatalogOptions<'a> {
    limit: usize,
    provider: Option<&'a str>,
    query: Option<String>,
}

impl<'a> CatalogOptions<'a> {
    pub(super) fn new(
        limit: Option<usize>,
        provider: Option<&'a str>,
        query: Option<&str>,
    ) -> anyhow::Result<Self> {
        let limit = limit.unwrap_or(25);
        anyhow::ensure!(
            (1..=100).contains(&limit),
            "For list_models, limit must be an integer from 1 to 100; omit it or use null for 25."
        );
        Ok(Self {
            limit,
            provider: provider.map(str::trim).filter(|value| !value.is_empty()),
            query: query
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_ascii_lowercase),
        })
    }

    fn matches(&self, route: &ModelRoute) -> bool {
        self.provider
            .is_none_or(|provider| provider_matches(route, provider))
            && self.query.as_ref().is_none_or(|query| {
                [&route.model, &route.provider, &route.api_method]
                    .iter()
                    .any(|field| field.to_ascii_lowercase().contains(query))
            })
    }
}

fn provider_matches(route: &ModelRoute, selector: &str) -> bool {
    // Account identity must never fall back to a family label or lose suffix case.
    // Match the raw API-method base, not credential aliases, for qualified routes.
    if let Some((base, account)) = selector.split_once('@') {
        return route
            .api_method
            .split_once('@')
            .is_some_and(|(route_base, route_account)| {
                route_base.eq_ignore_ascii_case(base) && route_account == account
            });
    }
    let method = route.api_method_kind();
    let key = RuntimeKey::from_api_method(&method, &route.provider).stable_id();
    if AuthRoute::parse_explicit_credential_prefix(selector).is_some() {
        return model_route_provider_matches_key(Some(&key), &route.provider, selector);
    }
    // Profile IDs are terminal: family-label normalization erases meaningful
    // punctuation (for example local-gpu versus local_gpu).
    if method.profile_id().is_some() {
        return method.matches_openai_compatible_profile(selector)
            || key.eq_ignore_ascii_case(selector);
    }
    key.eq_ignore_ascii_case(selector)
        || model_route_provider_labels_match(&route.provider, selector)
}

fn counts(incoming: usize, matched: usize, shown: usize, reason: &str) -> String {
    format!(
        "Catalog counts: incoming={incoming}, matched={matched}, shown={shown}, omitted={}, truncated={}, reason={reason}\n",
        matched - shown,
        shown < matched,
    )
}

fn route_row(route: &ModelRoute) -> String {
    let availability = if route.available {
        ""
    } else {
        " [unavailable]"
    };
    let cost = match route.estimated_reference_cost_micros() {
        Some(micros) => format!(" ~${:.2}/ref-task", micros as f64 / 1_000_000.0),
        None => String::new(),
    };
    let detail = if route.detail.is_empty() {
        String::new()
    } else {
        format!(" ({})", route.detail)
    };
    format!(
        "- {} via {} [{}]{}{}{}\n",
        route.model, route.provider, route.api_method, availability, cost, detail
    )
}

/// Preserve server order and complete route facts, bounding only the final tool text.
pub(super) fn format_swarm_model_list(
    current_model: Option<&str>,
    configured_swarm_model: Option<&str>,
    model_routes: &[ModelRoute],
    options: &CatalogOptions<'_>,
) -> String {
    let incoming = model_routes.len();
    let matching = || model_routes.iter().filter(|route| options.matches(route));
    let matched = matching().count();
    let mut out = format!(
        "Current coordinator model: {}\n",
        current_model.unwrap_or("unknown")
    );
    match configured_swarm_model {
        Some(pin) if !pin.trim().is_empty() => {
            out.push_str(&format!("Configured agents.swarm_model default: {pin}\n"));
        }
        _ => out.push_str(
            "No agents.swarm_model default configured (workers inherit the coordinator's model unless model is passed).\n",
        ),
    }
    out.push_str("\nModel routes (including unavailable routes):\n");
    // Each actual count has at most incoming's decimal width. Reserve the longest
    // reason and boolean, plus all status/footer text before admitting any row.
    let count_reserve = format!(
        "Catalog counts: incoming={incoming}, matched={incoming}, shown={incoming}, omitted={incoming}, truncated=false, reason=output_bytes\n"
    ).len();
    let tail_reserve =
        count_reserve + EMPTY.len().max(NO_MATCH.len()).max(NO_FIT.len()) + FOOTER.len();
    if out.len() + tail_reserve > MAX_OUTPUT_BYTES {
        return format!(
            "Model catalog header exceeds the 16384-byte display budget; zero routes displayed.\n{}Presentation only: daemon allocation and transfer are not bounded.\n",
            counts(incoming, matched, 0, "output_bytes")
        );
    }
    let mut shown = 0;
    let mut reason = "none";
    for route in matching() {
        if shown == options.limit {
            reason = "limit";
            break;
        }
        let row = route_row(route);
        if out.len() + row.len() + tail_reserve > MAX_OUTPUT_BYTES {
            reason = "output_bytes";
            break;
        }
        out.push_str(&row);
        shown += 1;
    }
    if incoming == 0 {
        out.push_str(EMPTY);
    } else if matched == 0 {
        out.push_str(NO_MATCH);
    } else if shown == 0 {
        out.push_str(NO_FIT);
    }
    out.push_str(&counts(incoming, matched, shown, reason));
    out.push_str(FOOTER);
    debug_assert!(out.len() <= MAX_OUTPUT_BYTES);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::communicate::CommunicateInput;
    use serde_json::json;

    fn route(model: &str, provider: &str, api_method: &str) -> ModelRoute {
        ModelRoute {
            model: model.to_string(),
            provider: provider.to_string(),
            api_method: api_method.to_string(),
            available: true,
            detail: String::new(),
            cheapness: None,
            usage: None,
        }
    }

    fn render(
        routes: &[ModelRoute],
        limit: Option<usize>,
        provider: Option<&str>,
        query: Option<&str>,
    ) -> String {
        let options = CatalogOptions::new(limit, provider, query).unwrap();
        let output = format_swarm_model_list(None, None, routes, &options);
        assert!(output.len() <= MAX_OUTPUT_BYTES);
        output
    }

    fn rows(output: &str) -> Vec<&str> {
        output
            .lines()
            .filter(|line| line.starts_with("- "))
            .collect()
    }

    #[test]
    fn ten_thousand_routes_preserve_default_and_maximum_counts_and_order() {
        let routes: Vec<_> = (0..10_000)
            .map(|i| route(&format!("model-{i:05}"), "OpenAI", "openai-oauth"))
            .collect();
        for (limit, shown) in [(None, 25), (Some(100), 100)] {
            let output = render(&routes, limit, None, None);
            let expected: Vec<_> = routes[..shown].iter().map(route_row).collect();
            assert_eq!(
                rows(&output),
                expected
                    .iter()
                    .map(|row| row.trim_end())
                    .collect::<Vec<_>>()
            );
            assert!(output.contains(&counts(10_000, 10_000, shown, "limit")));
        }
    }

    #[test]
    fn provider_aliases_preserve_credentials_profiles_and_account_identity() {
        let routes = vec![
            route("oa", "OpenAI", "openai-api-key"),
            route("oo", "OpenAI", "openai-oauth"),
            route("work", "OpenAI", "openai-oauth@Work"),
            route("aa", "Anthropic", "anthropic-api-key"),
            route("ao", "Anthropic", "claude-oauth"),
            route("go", "OpenCode Go", "openai-compatible:opencode-go"),
            route("router", "auto", "openrouter"),
            route("google", "Google", "code-assist-oauth"),
            route("bedrock", "AWS Bedrock", "bedrock"),
        ];
        for (selector, expected) in [
            ("openai", vec![0, 1, 2]),
            ("openai-api", vec![0]),
            ("OPENAI-API-KEY", vec![0]),
            ("openai-platform", vec![0]),
            ("openai-oauth", vec![1, 2]),
            ("openai-oauth@Work", vec![2]),
            ("OPENAI-OAUTH@Work", vec![2]),
            ("openai-oauth@work", vec![]),
            ("openai-oauth@Absent", vec![]),
            ("claude", vec![3, 4]),
            ("anthropic", vec![3, 4]),
            ("claude-api", vec![3]),
            ("anthropic-api", vec![3]),
            ("api-key", vec![3]),
            ("anthropic-oauth", vec![4]),
            ("opencode-go", vec![5]),
            ("OPENCODE-GO", vec![5]),
            ("openai-compatible:opencode-go", vec![5]),
            ("openrouter", vec![6]),
            ("gemini", vec![7]),
            ("bedrock", vec![8]),
            ("missing", vec![]),
            ("open", vec![]),
        ] {
            let output = render(&routes, None, Some(selector), None);
            let expected: Vec<_> = expected.iter().map(|i| route_row(&routes[*i])).collect();
            assert_eq!(
                rows(&output),
                expected
                    .iter()
                    .map(|row| row.trim_end())
                    .collect::<Vec<_>>(),
                "{selector}"
            );
        }
        let misleading_label = route("model", "openai-api", "openai-oauth");
        assert!(!provider_matches(&misleading_label, "openai-api"));
        let profile = routes[5].api_method_kind();
        assert!(profile.matches_openai_compatible_profile("opencode-go"));
        assert!(!provider_matches(&routes[5], "openai"));
    }

    #[test]
    fn compatible_profile_selectors_do_not_collapse_punctuation_or_fall_back_to_labels() {
        let routes = vec![
            route("hyphen", "local-gpu", "openai-compatible:local-gpu"),
            route("underscore", "local_gpu", "openai-compatible:local_gpu"),
        ];
        for (selector, index) in [
            ("local-gpu", 0),
            ("LOCAL-GPU", 0),
            ("openai-compatible:local-gpu", 0),
            ("local_gpu", 1),
            ("LOCAL_GPU", 1),
            ("openai-compatible:local_gpu", 1),
        ] {
            let output = render(&routes, None, Some(selector), None);
            assert_eq!(
                rows(&output),
                vec![route_row(&routes[index]).trim_end()],
                "{selector}"
            );
            assert!(output.contains(&counts(2, 1, 1, "none")));
        }
        for selector in [
            "localgpu",
            "local gpu",
            "openai-compatible:localgpu",
            "missing",
            "openai",
        ] {
            let output = render(&routes, None, Some(selector), None);
            assert!(rows(&output).is_empty(), "{selector}");
            assert!(output.contains(&counts(2, 0, 0, "none")));
        }
        let misleading_label = route("m", "OpenAI", "openai-compatible:local-gpu");
        assert!(!provider_matches(&misleading_label, "openai"));
        assert!(!provider_matches(&misleading_label, "openai-api"));
    }

    #[test]
    fn account_qualified_selectors_keep_raw_base_and_case_sensitive_account() {
        let routes = vec![
            route("claude", "Anthropic", "claude-oauth@Work"),
            route("openai", "OpenAI", "openai-oauth@Work"),
        ];
        for (selector, index) in [("claude-oauth@Work", 0), ("openai-oauth@Work", 1)] {
            assert_eq!(
                rows(&render(&routes, None, Some(selector), None)),
                vec![route_row(&routes[index]).trim_end()]
            );
        }
        for selector in [
            "anthropic-oauth@Work",
            "anthropic@Work",
            "claude@Work",
            "openai@Work",
            "claude-oauth@work",
            "openai-oauth@work",
        ] {
            assert!(
                rows(&render(&routes, None, Some(selector), None)).is_empty(),
                "{selector}"
            );
        }
        assert!(
            provider_matches(&routes[0], "anthropic-oauth"),
            "unqualified credential aliases remain supported"
        );
    }

    #[test]
    fn literal_ascii_query_is_anded_and_searches_only_the_three_fields() {
        let mut first = route("Model[.*]", "OpenAI", "openai-oauth@Work");
        first.detail = "secret detail-only".to_string();
        first.available = false;
        let routes = vec![
            first.clone(),
            first,
            route("Model[.*]", "Anthropic", "claude-oauth"),
        ];
        for query in ["[.*]", "MODEL[.*]", "oPeNaI", "@wOrK"] {
            let output = render(&routes, None, Some("openai"), Some(query));
            assert_eq!(rows(&output).len(), 2, "{query}");
            assert!(output.contains(&counts(3, 2, 2, "none")));
            assert_eq!(rows(&output)[0], rows(&output)[1], "no deduplication");
        }
        for query in [
            "model.*",
            "detail-only",
            "unavailable",
            "oauth|api",
            "Model OpenAI",
        ] {
            assert!(
                rows(&render(&routes, None, None, Some(query))).is_empty(),
                "{query}"
            );
        }
        let unicode = vec![route("Älpha", "OpenAI", "openai-oauth")];
        assert!(rows(&render(&unicode, None, None, Some("älpha"))).is_empty());
        assert_eq!(rows(&render(&unicode, None, None, Some("ÄLPHA"))).len(), 1);
    }

    #[test]
    fn omitted_null_and_blank_filters_and_limits_use_defaults() {
        let routes = vec![route("model", "OpenAI", "openai-oauth")];
        let expected = render(&routes, None, None, None);
        for input in [
            json!({"action": "list_models"}),
            json!({"action": "list_models", "limit": null, "provider": null, "query": null}),
            json!({"action": "list_models", "provider": " \t\n", "query": " \t\n"}),
        ] {
            let input: CommunicateInput = serde_json::from_value(input).unwrap();
            let options = CatalogOptions::new(
                input.limit,
                input.provider.as_deref(),
                input.query.as_deref(),
            )
            .unwrap();
            assert_eq!(options.limit, 25);
            assert_eq!(
                format_swarm_model_list(None, None, &routes, &options),
                expected
            );
        }
        assert_eq!(
            render(&routes, None, Some(" OpenAI "), Some(" MODEL ")),
            expected
        );
        for limit in [0, 101, usize::MAX] {
            assert!(CatalogOptions::new(Some(limit), None, None).is_err());
        }
        for limit in [json!(-1), json!(1.5), json!("25"), json!(true), json!([])] {
            assert!(
                serde_json::from_value::<CommunicateInput>(
                    json!({"action": "list_models", "limit": limit})
                )
                .is_err()
            );
        }
        for limit in [1, 100] {
            assert_eq!(
                CatalogOptions::new(Some(limit), None, None).unwrap().limit,
                limit
            );
        }
    }

    #[test]
    fn empty_catalog_and_zero_filter_matches_are_distinct_without_echoing_filters() {
        let empty = render(&[], None, None, None);
        assert!(empty.contains(EMPTY));
        assert!(empty.contains(&counts(0, 0, 0, "none")));
        assert!(!empty.contains(NO_MATCH));
        let unknown = "unknown".repeat(10_000);
        let no_match = render(
            &[route("m", "OpenAI", "openai-oauth")],
            None,
            Some(&unknown),
            None,
        );
        assert!(no_match.contains(NO_MATCH));
        assert!(no_match.contains(&counts(1, 0, 0, "none")));
        assert!(!no_match.contains(EMPTY));
        assert!(!no_match.contains("unknownunknown"));
    }

    #[test]
    fn route_without_cheapness_has_no_cost_suffix() {
        let model = route("gpt-5.5", "OpenAI", "openai-oauth@Work");
        assert!(model.cheapness.is_none());
        assert_eq!(
            route_row(&model),
            "- gpt-5.5 via OpenAI [openai-oauth@Work]\n"
        );
    }

    #[test]
    fn preserves_defaults_inherit_unavailability_raw_auth_detail_and_cost() {
        use jcode_provider_core::{RouteCheapnessEstimate, RouteCostConfidence, RouteCostSource};
        let mut model = route("gpt-5.5", "OpenAI", "openai-oauth@Work");
        model.available = false;
        model.detail = "Saved Work account; unavailable until reset".to_string();
        model.cheapness = Some(RouteCheapnessEstimate::metered(
            RouteCostSource::PublicApiPricing,
            RouteCostConfidence::Exact,
            1_000_000,
            5_000_000,
            None,
            None,
        ));
        let options = CatalogOptions::new(None, None, None).unwrap();
        let output = format_swarm_model_list(
            Some("claude-fable-5"),
            Some("openai-api:gpt-5.5"),
            &[model],
            &options,
        );
        assert!(output.len() <= MAX_OUTPUT_BYTES);
        assert!(output.contains("Current coordinator model: claude-fable-5\n"));
        assert!(output.contains("Configured agents.swarm_model default: openai-api:gpt-5.5\n"));
        assert!(output.contains("- gpt-5.5 via OpenAI [openai-oauth@Work] [unavailable] ~$0.05/ref-task (Saved Work account; unavailable until reset)\n"));
        assert!(output.contains("pass inherit to use the coordinator"));
        assert!(output.contains("effort"));
        assert!(output.contains("daemon allocation and transfer are not bounded"));
        let output = format_swarm_model_list(None, Some(" \t"), &[], &options);
        assert!(output.contains("Current coordinator model: unknown"));
        assert!(output.contains("No agents.swarm_model default configured"));
        assert!(output.contains("unless model is passed"));
    }

    #[test]
    fn long_multibyte_rows_stop_at_first_nonfitting_row_without_partial_facts() {
        for detail in ["x".repeat(1_000), "界".repeat(1_000)] {
            let routes: Vec<_> = (0..25)
                .map(|i| {
                    let mut model = route(&format!("model-{i}"), "OpenAI", "openai-oauth@Work");
                    model.available = false;
                    model.detail = format!("{detail}-end-{i}");
                    model
                })
                .collect();
            let output = render(&routes, None, None, None);
            let shown = rows(&output).len();
            assert!(shown > 0 && shown < 25);
            assert!(output.contains(&counts(25, 25, shown, "output_bytes")));
            for (i, model) in routes.iter().enumerate() {
                assert_eq!(output.contains(&route_row(model)), i < shown);
            }
            assert!(!output.contains(&format!("- model-{shown} via")));
        }
    }

    #[test]
    fn oversized_first_row_is_not_no_matches_or_repaired_by_lower_limit() {
        let mut huge = route("oversized", "OpenAI", "openai-oauth@Work");
        huge.detail = "界".repeat(MAX_OUTPUT_BYTES);
        for limit in [None, Some(1), Some(100)] {
            let output = render(
                &[huge.clone(), route("small", "OpenAI", "openai-oauth")],
                limit,
                None,
                None,
            );
            assert!(output.contains(NO_FIT));
            assert!(output.contains(&counts(2, 2, 0, "output_bytes")));
            assert!(!output.contains(NO_MATCH));
            assert!(!output.contains(EMPTY));
            assert!(!output.contains("lower"));
            assert!(rows(&output).is_empty());
        }
    }

    #[test]
    fn header_overflow_is_bounded_for_current_model_and_configured_default() {
        let huge = "界".repeat(MAX_OUTPUT_BYTES);
        let options = CatalogOptions::new(None, None, None).unwrap();
        for (current, pin) in [(Some(huge.as_str()), None), (None, Some(huge.as_str()))] {
            let output = format_swarm_model_list(
                current,
                pin,
                &[route("m", "OpenAI", "openai-oauth")],
                &options,
            );
            assert!(output.len() <= MAX_OUTPUT_BYTES);
            assert!(output.contains("header exceeds"));
            assert!(output.contains(&counts(1, 1, 0, "output_bytes")));
            assert!(rows(&output).is_empty());
            assert!(!output.contains('界'));
        }
    }
}
