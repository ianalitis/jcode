// Included by `communicate.rs`; keeps the model-catalog renderer out of an
// already-oversized file. Paths stay relative to the including module.
/// Detailed route lines emitted before the listing switches to a truncation
/// notice. The live catalog can hold ~1000 routes, which costs tens of thousands
/// of tokens of agent context for information a caller rarely needs in full;
/// `query` narrows it instead.
const SWARM_MODEL_LIST_DETAIL_LIMIT: usize = 60;

/// Render the swarm model catalog for the `list_models` action: the current
/// (spawn-default) model, any config pin, and one line per matching route with
/// availability, auth method, and a relative cost estimate.
///
/// `query` filters case-insensitively over model id, provider, and api method.
fn format_swarm_model_list(
    current_model: Option<&str>,
    configured_swarm_model: Option<&str>,
    model_routes: &[jcode_provider_core::ModelRoute],
    query: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Current coordinator model: {}\n",
        current_model.unwrap_or("unknown")
    ));
    match configured_swarm_model {
        Some(pin) if !pin.trim().is_empty() => {
            out.push_str(&format!("Configured agents.swarm_model default: {pin}\n"));
        }
        _ => out.push_str(
            "No agents.swarm_model default configured (workers inherit the coordinator's model unless model is passed).\n",
        ),
    }
    if model_routes.is_empty() {
        out.push_str("\nNo model routes reported. Omit model to use the configured default, or pass inherit to use the coordinator.");
        return out;
    }

    let query = query
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let matching: Vec<&jcode_provider_core::ModelRoute> = model_routes
        .iter()
        .filter(|route| {
            let Some(query) = query.as_deref() else {
                return true;
            };
            route.model.to_ascii_lowercase().contains(query)
                || route.provider.to_ascii_lowercase().contains(query)
                || route.api_method.to_ascii_lowercase().contains(query)
        })
        .collect();

    match query.as_deref() {
        Some(query) => out.push_str(&format!(
            "\nModel routes matching \"{query}\" ({} of {}):\n",
            matching.len(),
            model_routes.len()
        )),
        None => out.push_str(&format!(
            "\nAvailable model routes ({} total; pass model to override the configured default, or query to filter by model, provider, or auth method):\n",
            model_routes.len()
        )),
    }
    if matching.is_empty() {
        out.push_str("- none. Try a shorter query such as a provider name.\n");
    }

    let mut shown = 0usize;
    for route in matching.iter().take(SWARM_MODEL_LIST_DETAIL_LIMIT) {
        shown += 1;
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
        out.push_str(&format!(
            "- {} via {} [{}]{}{}{}\n",
            route.model, route.provider, route.api_method, availability, cost, detail
        ));
    }
    if matching.len() > shown {
        out.push_str(&format!(
            "- ... {} more matching route(s) omitted; narrow with query.\n",
            matching.len() - shown
        ));
    }
    out.push_str("\nAlso pass effort (none|minimal|low|medium|high|xhigh|max) to set the spawned agent's reasoning effort.");
    out
}
