use anyhow::Result;
use serde_json::Value;

/// Render a product selection. Catalog selections contain a full `tool` entry
/// and return its setup instructions. Off-catalog selections contain receipt
/// metadata but no provider or setup fields: they are acknowledged for demand
/// attribution without inventing, fetching, or endorsing provider data.
pub(super) fn render_selection(category: &str, tool_name: &str, listing: &Value) -> Result<String> {
    let receipt_category = listing
        .get("category")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("discovery selection receipt omitted its category"))?;
    if !receipt_category.eq_ignore_ascii_case(category) {
        return Err(anyhow::anyhow!(
            "discovery selection receipt category '{receipt_category}' did not match requested category '{category}'"
        ));
    }
    let selected_tool = listing
        .get("selected_tool")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("discovery selection receipt omitted the selected product")
        })?;
    if !selected_tool.eq_ignore_ascii_case(tool_name) {
        return Err(anyhow::anyhow!(
            "discovery selection receipt named '{selected_tool}', not requested product '{tool_name}'"
        ));
    }
    let listed = listing
        .get("listed")
        .and_then(Value::as_bool)
        .ok_or_else(|| anyhow::anyhow!("discovery selection receipt omitted catalog status"))?;

    if !listed {
        for forbidden in ["tool", "provider", "setup", "url", "mcp"] {
            if listing.get(forbidden).is_some() {
                return Err(anyhow::anyhow!(
                    "off-catalog selection receipt for '{selected_tool}' unexpectedly included provider field '{forbidden}'"
                ));
            }
        }
        return Ok(format!(
            "Selected off-catalog product '{selected_tool}' for '{category}'.\n\n\
             Selection recorded as demand data. Jcode does not list an integration for this \
             product, so no provider information, recommendation, or setup instructions \
             are provided. Continue using only information independently available to you."
        ));
    }

    let tool = listing
        .get("tool")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!("catalog selection receipt contained no provider details")
        })?;
    let name = tool
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("catalog selection receipt omitted the provider name"))?;
    if !name.eq_ignore_ascii_case(tool_name) || !name.eq_ignore_ascii_case(selected_tool) {
        return Err(anyhow::anyhow!(
            "catalog provider name '{name}' did not match selected product '{selected_tool}'"
        ));
    }
    let setup = tool
        .get("setup")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("catalog selection receipt for '{name}' omitted setup instructions")
        })?;
    let blurb = tool.get("blurb").and_then(|v| v.as_str()).unwrap_or("");
    let mut out = format!(
        "Selected '{name}' from '{category}' (Jcode integration directory; the choice must be based only \
         on fit; details: {}):\n\n{name}: {blurb}",
        crate::sponsors::DISCOVERY_PARTNERS_URL
    );
    if let Some(url) = tool.get("url").and_then(|v| v.as_str()) {
        out.push_str(&format!(" ({url})"));
    }
    out.push_str(&format!("\n\nSetup: {setup}"));
    out.push_str(
        "\n\nConsequential actions (signups, spending) must note that setup is provided through a Jcode integration in \
         the confirmation shown to the user.",
    );
    Ok(out)
}
