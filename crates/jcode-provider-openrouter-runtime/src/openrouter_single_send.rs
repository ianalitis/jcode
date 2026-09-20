// Included by `openrouter_provider_impl.rs`. Holds the constrained single-send
// request helpers so the provider implementation file stays inside its size
// budget; because it is included, the parent module's imports remain in scope.
pub(super) fn validate_expected_final_request(expected: &Value, actual: &Value) -> Result<()> {
    anyhow::ensure!(
        expected == actual,
        "final OpenRouter request differs from the trusted expected request"
    );
    Ok(())
}

pub(super) fn merge_extra_body_and_validate(
    request: &mut Value,
    extra_body: Option<&serde_json::Map<String, Value>>,
    expected_final_request: Option<&Value>,
) -> Result<()> {
    if let Some(extra) = extra_body
        && let Some(request_obj) = request.as_object_mut()
    {
        for (key, value) in extra {
            request_obj.insert(key.clone(), value.clone());
        }
    }

    if let Some(expected) = expected_final_request {
        validate_expected_final_request(expected, request)?;
    }
    Ok(())
}

fn single_send_destination_is_allowed(destination: &reqwest::Url) -> bool {
    if destination.scheme() == "https" {
        return true;
    }

    #[cfg(test)]
    {
        destination.scheme() == "http"
            && destination.host_str().is_some_and(|host| {
                host.eq_ignore_ascii_case("localhost")
                    || host
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|address| address.is_loopback())
            })
    }

    #[cfg(not(test))]
    false
}

fn prepare_single_send_transport(
    api_base: &str,
    expected_destination: &str,
) -> Result<(Client, String)> {
    let destination = reqwest::Url::parse(expected_destination)
        .context("invalid trusted single-send OpenRouter destination")?;
    anyhow::ensure!(
        single_send_destination_is_allowed(&destination),
        "trusted single-send OpenRouter destination must use HTTPS"
    );
    anyhow::ensure!(
        destination.username().is_empty()
            && destination.password().is_none()
            && destination.fragment().is_none(),
        "trusted single-send OpenRouter destination contains forbidden URL components"
    );

    let actual_destination = chat_completions_url(api_base);
    anyhow::ensure!(
        expected_destination == actual_destination,
        "single-send OpenRouter destination differs from the trusted expected destination"
    );

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .retry(reqwest::retry::never())
        .build()
        .context("failed to build constrained single-send OpenRouter client")?;
    Ok((client, actual_destination))
}
