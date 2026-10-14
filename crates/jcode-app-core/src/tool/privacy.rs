//! Apply the shared text/JSON boundary without changing tool execution inputs.
use super::{Registry, ToolContext, ToolOutput};
use anyhow::Result;
use jcode_base::message::{sanitize_outbound_json, sanitize_outbound_text};
use serde_json::Value;

impl Registry {
    /// Fire the `post_tool` observer hook with tool outcome metadata.
    /// No-op (without building the payload) when the hook is not configured.
    pub(super) fn fire_post_tool_hook(
        resolved_name: &str,
        input: &Value,
        ctx: &ToolContext,
        result: &Result<ToolOutput>,
        latency_ms: u64,
    ) {
        if !crate::hooks::hook_configured("post_tool") {
            return;
        }
        crate::hooks::dispatch_observer(Self::post_tool_event(
            "post_tool",
            resolved_name,
            input,
            ctx,
            result.as_ref(),
            latency_ms,
        ));
    }
}

pub(crate) fn sanitize_tool_output(mut output: ToolOutput) -> Result<ToolOutput> {
    output.output = sanitize_outbound_text(&output.output)?;
    if let Some(title) = &mut output.title {
        *title = sanitize_outbound_text(title)?;
    }
    if let Some(metadata) = &mut output.metadata {
        sanitize_outbound_json(metadata)?;
    }
    for image in &mut output.images {
        if let Some(label) = &mut image.label {
            *label = sanitize_outbound_text(label)?;
        }
        // Encoded image bytes are not text and cannot be meaningfully scrubbed
        // here. Do not corrupt them or claim that they have been inspected.
    }
    Ok(output)
}

pub(super) fn sanitize_tool_result(result: Result<ToolOutput>) -> Result<ToolOutput> {
    match result {
        Ok(output) => sanitize_tool_output(output),
        // Flatten the sanitized chain. Retaining the original error as a source
        // would leak it through alternate formatting, logs, or hook payloads.
        Err(error) => Err(anyhow::anyhow!(sanitize_outbound_text(&format!(
            "{error:#}"
        ))?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> String {
        ["4111", "1111", "1111", "1111"].join("")
    }

    #[test]
    fn privacy_covers_text_title_metadata_labels_and_error_chains() {
        let sensitive = card();
        let output = ToolOutput::new(&sensitive)
            .with_title(&sensitive)
            .with_metadata(serde_json::json!({"nested": [sensitive]}))
            .with_labeled_image("image/png", "encoded-image", &sensitive);
        let clean = sanitize_tool_output(output).unwrap();
        assert!(!format!("{clean:?}").contains(&sensitive));
        assert_eq!(clean.images[0].data, "encoded-image");
        let error = anyhow::anyhow!(sensitive.clone()).context(format!("outer {sensitive}"));
        let clean = sanitize_tool_result(Err(error)).unwrap_err();
        assert!(!format!("{clean:#}").contains(&sensitive));
        assert!(clean.source().is_none());
    }

    #[test]
    fn privacy_rechecks_feedback_and_fails_closed() {
        let mut output = sanitize_tool_output(ToolOutput::new("ordinary")).unwrap();
        output
            .output
            .push_str(&format!("\n[post_tool_feedback]\n{}", card()));
        let clean = sanitize_tool_output(output).unwrap();
        assert!(!clean.output.contains(&card()));
        let mut nested = serde_json::Value::Null;
        for _ in 0..70 {
            nested = serde_json::json!([nested]);
        }
        let error =
            sanitize_tool_output(ToolOutput::new(card()).with_metadata(nested)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "outbound sanitizer failed; content withheld"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn privacy_registry_sanitizes_before_hooks_and_after_feedback() {
        use super::super::{Registry, Tool, ToolContext, ToolExecutionMode};
        use serde_json::Value;
        use std::sync::Arc;

        struct Echo;
        #[async_trait::async_trait]
        impl Tool for Echo {
            fn name(&self) -> &str {
                "privacy_echo"
            }
            fn description(&self) -> &str {
                "Privacy boundary regression"
            }
            fn parameters_schema(&self) -> Value {
                serde_json::json!({"type": "object"})
            }
            async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
                assert!(
                    input["value"] == card(),
                    "local execution input was changed"
                );
                if input["fail"] == true {
                    anyhow::bail!(card());
                }
                Ok(ToolOutput::new(card()))
            }
        }
        struct HookEnv(Vec<(&'static str, Option<std::ffi::OsString>)>);
        impl Drop for HookEnv {
            fn drop(&mut self) {
                for (key, value) in &self.0 {
                    match value {
                        Some(value) => crate::env::set_var(key, value),
                        None => crate::env::remove_var(key),
                    }
                }
            }
        }
        let _env_lock = crate::storage::lock_test_env();
        let dir = tempfile::tempdir().unwrap();
        let observer = dir.path().join("observer.sh");
        std::fs::write(&observer, "#!/bin/sh\ncat >/dev/null\nprintf '%s:%s:%s' \"$JCODE_HOOK_STATUS\" \"$JCODE_HOOK_OUTPUT_BYTES\" \"$JCODE_HOOK_ERROR\" > \"observer-$JCODE_HOOK_TOOL_CALL_ID\"\n").unwrap();
        let feedback = dir.path().join("feedback.sh");
        std::fs::write(
            &feedback,
            "#!/bin/sh\ncat > feedback-input\nprintf '%s%s%s%s' '4111' '1111' '1111' '1111'\n",
        )
        .unwrap();
        let vars = ["JCODE_HOOK_POST_TOOL", "JCODE_HOOK_POST_TOOL_FEEDBACK"];
        let _env = HookEnv(
            vars.iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect(),
        );
        crate::env::set_var(vars[0], format!("/bin/sh {}", observer.display()));
        crate::env::set_var(vars[1], format!("/bin/sh {}", feedback.display()));
        let registry = Registry::empty();
        registry
            .register("privacy_echo".into(), Arc::new(Echo))
            .await;
        for fail in [false, true] {
            let call = if fail { "error" } else { "ok" };
            let ctx = ToolContext {
                session_id: "privacy-test".into(),
                message_id: "message".into(),
                tool_call_id: call.into(),
                working_dir: Some(dir.path().into()),
                stdin_request_tx: None,
                graceful_shutdown_signal: None,
                execution_mode: ToolExecutionMode::Direct,
            };
            let result = registry
                .execute(
                    "privacy_echo",
                    serde_json::json!({"value": card(), "fail": fail}),
                    ctx,
                )
                .await;
            if fail {
                assert!(!format!("{:#}", result.unwrap_err()).contains(&card()));
            } else {
                let output = result.unwrap().output;
                assert!(!output.contains(&card()));
                assert!(output.contains("[post_tool_feedback]"));
                let input = std::fs::read_to_string(dir.path().join("feedback-input")).unwrap();
                assert!(!input.contains(&card()));
            }
            let path = dir.path().join(format!("observer-{call}"));
            let observed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    if let Ok(text) = std::fs::read_to_string(&path)
                        && !text.is_empty()
                    {
                        break text;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            assert!(!observed.contains(&card()));
            if fail {
                assert!(observed.contains("[REDACTED_CARD]"));
            } else {
                assert!(observed == format!("ok:{}:", "[REDACTED_CARD]".len()));
            }
        }
    }
}
