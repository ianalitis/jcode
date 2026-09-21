use crate::message::{ContentBlock, ToolCall};
use crate::terminal_println as println;
use crate::tool::ToolOutput;

/// Floor for [`history_output_cap_chars`]. A configured zero or tiny value must
/// not truncate every tool output down to nothing.
const MIN_TOOL_OUTPUT_CHARS_FOR_HISTORY: usize = 1_024;

/// Chars of a single tool result kept in session history before the remainder is
/// spilled to disk. Configured by `tools.history_output_cap_chars` (env override
/// `JCODE_TOOL_HISTORY_OUTPUT_CAP_CHARS`), floored at 1 KiB.
fn history_output_cap_chars() -> usize {
    crate::config::config()
        .tools
        .history_output_cap_chars
        .max(MIN_TOOL_OUTPUT_CHARS_FOR_HISTORY)
}

/// Advice appended when the full text could not be spilled to disk.
const TRUNCATION_FALLBACK_ADVICE: &str =
    " Redirect large logs to a file and read targeted sections.";

/// Advice appended when the full text is available on disk, so the caller can
/// read exactly the section it needs instead of re-running the command.
fn spill_continuation_advice(path: &std::path::Path, original_chars: usize) -> String {
    format!(
        " Full output ({} chars) is saved at {}; read that path with offset/limit instead of re-running the command.",
        original_chars,
        path.display()
    )
}

pub(super) fn cap_tool_output_for_history(
    session_id: &str,
    tool_name: &str,
    mut output: ToolOutput,
) -> ToolOutput {
    let cap = history_output_cap_chars();
    if output.output.chars().count() <= cap {
        return output;
    }

    let original_chars = output.output.chars().count();
    let kept = crate::util::truncate_str(&output.output, cap);
    let advice = match super::tool_output_spill::spill_truncated_output(
        session_id,
        tool_name,
        &output.output,
    ) {
        Some(path) => spill_continuation_advice(&path, original_chars),
        None => TRUNCATION_FALLBACK_ADVICE.to_string(),
    };
    output.output = format!(
        "{}\n\n[Tool output truncated by jcode: tool `{}` produced {} chars; kept first {} chars to protect the remote protocol, session history, and prompt cache.{}]",
        kept, tool_name, original_chars, cap, advice,
    );
    output
}

pub(super) fn cap_sdk_tool_content_for_history(
    session_id: &str,
    tool_name: &str,
    content: String,
) -> String {
    let cap = history_output_cap_chars();
    if content.chars().count() <= cap {
        return content;
    }
    let original_chars = content.chars().count();
    let kept = crate::util::truncate_str(&content, cap);
    let advice =
        match super::tool_output_spill::spill_truncated_output(session_id, tool_name, &content) {
            Some(path) => spill_continuation_advice(&path, original_chars),
            None => TRUNCATION_FALLBACK_ADVICE.to_string(),
        };
    format!(
        "{}\n\n[Tool output truncated by jcode: tool `{}` produced {} chars; kept first {} chars to protect the remote protocol, session history, and prompt cache.{}]",
        kept, tool_name, original_chars, cap, advice,
    )
}

/// Build rendered side-pane images from a tool output's attached images.
///
/// This mirrors how `render_messages_and_images` derives images from persisted
/// session history (source = ToolResult), so live-streamed images match what a
/// later History reload would produce. `tool_name` and `tool_input` provide the
/// label fallback (e.g. the `read` tool's `file_path`); `tool_call_id` anchors
/// the image to its tool message in the transcript.
pub(super) fn tool_output_side_pane_images(
    tool_call_id: &str,
    tool_name: &str,
    tool_input: &serde_json::Value,
    output: &ToolOutput,
) -> Vec<jcode_session_types::RenderedImage> {
    if output.images.is_empty() {
        return Vec::new();
    }
    let fallback_label = tool_input
        .get("file_path")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    output
        .images
        .iter()
        .map(|img| jcode_session_types::RenderedImage {
            history_message_index: None,
            media_type: img.media_type.clone(),
            data: img.data.clone(),
            label: img
                .label
                .as_ref()
                .map(|label| label.trim().to_string())
                .filter(|label| !label.is_empty())
                .or_else(|| fallback_label.clone()),
            source: jcode_session_types::RenderedImageSource::ToolResult {
                tool_name: tool_name.to_string(),
            },
            anchor: Some(jcode_session_types::RenderedImageAnchor::ToolCall {
                id: tool_call_id.to_string(),
            }),
        })
        .collect()
}

pub(super) fn tool_output_to_content_blocks(
    tool_use_id: String,
    output: ToolOutput,
) -> Vec<ContentBlock> {
    let mut blocks = vec![ContentBlock::ToolResult {
        tool_use_id,
        content: output.output,
        is_error: None,
    }];
    for img in output.images {
        blocks.push(ContentBlock::Image {
            media_type: img.media_type,
            data: img.data,
        });
        if let Some(label) = img.label.filter(|label| !label.trim().is_empty()) {
            blocks.push(ContentBlock::Text {
                text: format!(
                    "[Attached image associated with the preceding tool result: {}]",
                    label
                ),
                cache_control: None,
            });
        }
    }
    blocks
}

pub(super) fn print_tool_summary(tool: &ToolCall) {
    match tool.name.as_str() {
        "bash" => {
            if let Some(cmd) = tool.input.get("command").and_then(|v| v.as_str()) {
                let short = if cmd.len() > 60 {
                    format!("{}...", crate::util::truncate_str(cmd, 60))
                } else {
                    cmd.to_string()
                };
                println!("$ {}", short);
            }
        }
        "read" | "write" | "edit" => {
            if let Some(path) = tool.input.get("file_path").and_then(|v| v.as_str()) {
                println!("{}", path);
            }
        }
        "glob" | "grep" => {
            if let Some(pattern) = tool.input.get("pattern").and_then(|v| v.as_str()) {
                println!("'{}'", pattern);
            }
        }
        "ls" => {
            let path = tool
                .input
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            println!("{}", path);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Point `JCODE_HOME` at a temp dir so the spill lands there and is cleaned
    /// up with the guard.
    struct SpillHome {
        previous: Option<std::ffi::OsString>,
        dir: tempfile::TempDir,
    }

    impl SpillHome {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().expect("temp dir");
            let previous = std::env::var_os("JCODE_HOME");
            crate::env::set_var("JCODE_HOME", dir.path());
            Self { previous, dir }
        }
    }

    impl Drop for SpillHome {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => crate::env::set_var("JCODE_HOME", value),
                None => crate::env::remove_var("JCODE_HOME"),
            }
        }
    }

    #[test]
    fn authoritative_diff_text_survives_history_conversion_and_serialization() {
        let text =
            "Edited f\n\nFile diff:\n```diff\n--- f\n+++ f\n@@ -39,1 +39,1 @@\n-old\n+new\n```\n";
        let blocks = tool_output_to_content_blocks(
            "edit-call".into(),
            cap_tool_output_for_history("test-session", "edit", ToolOutput::new(text)),
        );
        let serialized = serde_json::to_string(&blocks).unwrap();
        let restored: Vec<ContentBlock> = serde_json::from_str(&serialized).unwrap();
        assert!(
            matches!(&restored[0], ContentBlock::ToolResult { content, tool_use_id, .. }
            if content == text && tool_use_id == "edit-call")
        );
    }

    #[test]
    fn cap_tool_output_leaves_small_output_unchanged() {
        let _lock = crate::storage::lock_test_env();
        let _home = SpillHome::new();
        let output = ToolOutput::new("short output");
        let capped = cap_tool_output_for_history("session_a", "bash", output.clone());
        assert_eq!(capped.output, output.output);
        assert!(
            !_home.dir.path().join("tool-output").exists(),
            "an untruncated output must not create a spill"
        );
    }

    #[test]
    fn cap_tool_output_adds_visible_truncation_notice() {
        let _lock = crate::storage::lock_test_env();
        let _home = SpillHome::new();
        let full = "x".repeat(history_output_cap_chars() + 10);
        let output = ToolOutput::new(full.clone());
        let capped = cap_tool_output_for_history("session_a", "bash", output);
        assert!(capped.output.len() < history_output_cap_chars() + 1_000);
        assert!(capped.output.contains("Tool output truncated by jcode"));
        assert!(capped.output.contains("tool `bash` produced"));
        assert!(
            !capped.output.contains("Redirect large logs to a file"),
            "a successful spill replaces the generic advice"
        );

        // The discarded bytes stay reachable: the notice names the file and the
        // file holds the whole output.
        let saved = capped
            .output
            .split("is saved at ")
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .expect("notice names the spilled path");
        let saved_path = std::path::Path::new(saved.trim());
        assert!(saved_path.exists(), "{saved}");
        assert_eq!(
            std::fs::read_to_string(saved_path).expect("read spill"),
            full
        );
    }

    #[test]
    fn cap_sdk_tool_content_adds_same_notice() {
        let _lock = crate::storage::lock_test_env();
        let _home = SpillHome::new();
        let capped = cap_sdk_tool_content_for_history(
            "session_a",
            "custom",
            "y".repeat(history_output_cap_chars() + 10),
        );
        assert!(capped.contains("Tool output truncated by jcode"));
        assert!(capped.contains("tool `custom` produced"));
        assert!(capped.contains("is saved at"));
    }

    #[test]
    fn cap_tool_output_honors_default_64k_cap() {
        let _lock = crate::storage::lock_test_env();
        let _home = SpillHome::new();
        // The temp JCODE_HOME holds no config.toml, so the shipped default applies.
        assert_eq!(history_output_cap_chars(), 65_536);

        let exact = "e".repeat(65_536);
        let capped =
            cap_tool_output_for_history("session_a", "bash", ToolOutput::new(exact.clone()));
        assert_eq!(capped.output, exact, "exactly at the cap must pass through");
        assert!(
            !_home.dir.path().join("tool-output").exists(),
            "an untruncated output must not create a spill"
        );

        let over = "o".repeat(65_537);
        let capped =
            cap_tool_output_for_history("session_a", "bash", ToolOutput::new(over.clone()));
        assert!(
            capped.output.contains("kept first 65536 chars"),
            "notice states the effective cap: {}",
            capped.output
        );
        let saved = capped
            .output
            .split("is saved at ")
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .expect("notice names the spilled path");
        let saved_path = std::path::Path::new(saved.trim());
        assert!(saved_path.exists(), "{saved}");
        assert_eq!(
            std::fs::read_to_string(saved_path).expect("read spill"),
            over
        );
    }
}

#[cfg(test)]
mod image_anchor_tests {
    use super::*;

    #[test]
    fn live_batch_images_anchor_to_parent_and_have_no_history_boundary() {
        let output = ToolOutput::new("batch results")
            .with_labeled_image("image/png", "one", "first.png")
            .with_labeled_image("image/png", "two", "second.png");
        let images =
            tool_output_side_pane_images("parent-batch", "batch", &serde_json::json!({}), &output);
        assert_eq!(images.len(), 2);
        for image in &images {
            assert_eq!(
                image.anchor,
                Some(jcode_session_types::RenderedImageAnchor::ToolCall {
                    id: "parent-batch".into()
                })
            );
            assert_eq!(image.history_message_index, None);
        }
        assert_eq!(images[0].data, "one");
        assert_eq!(images[1].data, "two");
        assert_eq!(images[0].label.as_deref(), Some("first.png"));
    }
}
