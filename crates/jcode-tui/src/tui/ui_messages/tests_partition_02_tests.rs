#[test]
fn render_tool_message_shows_intent_and_technical_preview_on_one_line() {
    crate::tui::ui::tools_ui::tests_tool_call_details_override::set(true);
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "ok".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_intent".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "cargo test -p jcode render_background_task --lib",
                "intent": "Verify compact progress card"
            }),
            intent: Some("Verify compact progress card".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered = extract_line_text(&lines[0]);

    assert!(rendered.contains("bash · Verify compact progress card · $ cargo test"));
    assert_eq!(lines.len(), 1, "Bash output is hidden by default");
    crate::tui::ui::tools_ui::tests_tool_call_details_override::set(false);
}

/// Default (tool_call_details off): a row with an intent renders only the
/// intent; the dimmed technical preview is dropped and no fallback command
/// line is added.
#[test]
fn render_tool_message_hides_technical_preview_by_default() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "ok".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_intent".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "cargo test -p jcode render_background_task --lib",
                "intent": "Verify compact progress card"
            }),
            intent: Some("Verify compact progress card".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered = extract_line_text(&lines[0]);

    assert!(
        rendered.contains("bash · Verify compact progress card"),
        "rendered={rendered}"
    );
    assert!(
        !rendered.contains("cargo test"),
        "technical detail should be hidden by default: {rendered}"
    );
    assert_eq!(lines.len(), 1, "Bash output is hidden by default");
}

/// Even with details off, a failed tool row keeps its error summary so
/// failures stay diagnosable.
#[test]
fn render_tool_message_keeps_error_summary_when_details_hidden() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "Error: command not found: cargoo".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_intent_err".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "cargoo test",
                "intent": "Run the test suite"
            }),
            intent: Some("Run the test suite".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered = extract_line_text(&lines[0]);

    assert!(
        rendered.contains("Run the test suite ·"),
        "error summary should still render after the intent: {rendered}"
    );
}

#[test]
fn render_tool_message_shows_token_badge() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "x".repeat(7_600),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_2".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let badge_span = lines[0]
        .spans
        .iter()
        .find(|span| span.content.contains("1.9k tok"))
        .expect("missing token badge");

    assert_eq!(badge_span.style.fg, Some(rgb(118, 118, 118)));
}

#[test]
fn render_tool_message_hides_bash_output() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "<class 'zip'>\n[('p', 'b'), ('a', 'a'), ('l', 'l'), ('e', 'e')]".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_bash_output".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({
                "command": "python3 -c \"s='pale'; t='bale'; print(type(zip(s,t))); print(list(zip(s,t)))\""
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let rendered = lines.iter().map(extract_line_text).collect::<Vec<_>>();

    assert!(!rendered.iter().any(|line| line.contains("<class 'zip'>")));
    assert!(!rendered.iter().any(|line| line.contains("[('p', 'b')")));
}

#[test]
fn render_tool_message_shows_bash_output_when_enabled() {
    crate::tui::ui::tools_ui::tests_show_bash_output_override::set(true);
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "one\ntwo\nthree\nfour".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_bash_output_enabled".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "printf output"}),
            intent: Some("Print output".to_string()),
            thought_signature: None,
        }),
    };

    let rendered = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>();

    assert_eq!(rendered.len(), 4);
    assert!(!rendered.iter().any(|line| line.trim() == "one"));
    assert!(rendered.iter().any(|line| line.trim() == "two"));
    assert!(rendered.iter().any(|line| line.trim() == "four"));
    crate::tui::ui::tools_ui::tests_show_bash_output_override::set(false);
}

fn gmail_draft_message(content: &str, input: serde_json::Value) -> DisplayMessage {
    DisplayMessage {
        role: "tool".to_string(),
        content: content.to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_gmail_draft".to_string(),
            name: "gmail".to_string(),
            input,
            intent: None,
            thought_signature: None,
        }),
    }
}

#[test]
fn render_tool_message_shows_gmail_draft_card() {
    let msg = gmail_draft_message(
        "Draft created successfully.\nDraft ID: draft_123\nTo: bob@example.com\nSubject: Project update",
        serde_json::json!({
            "action": "draft",
            "to": "bob@example.com",
            "subject": "Project update",
            "body": "Hi Bob,\n\nThe release is ready for review."
        }),
    );

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("Gmail draft created · draft_123"), "{plain}");
    assert!(
        !plain.contains('✉'),
        "draft card should not show an icon: {plain}"
    );
    assert!(plain.contains("To: bob@example.com"), "{plain}");
    assert!(plain.contains("Subject: Project update"), "{plain}");
    assert!(
        plain.contains("The release is ready for review."),
        "{plain}"
    );
    assert!(
        !plain.contains("\"body\""),
        "must not leak raw JSON: {plain}"
    );
}

#[test]
fn render_gmail_draft_card_marks_failures_and_empty_fields() {
    let msg = gmail_draft_message(
        "Error: Gmail draft creation failed",
        serde_json::json!({ "action": "draft", "body": "" }),
    );

    let lines = render_tool_message(&msg, 80, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("Gmail draft failed"), "{plain}");
    assert!(plain.contains("(recipient missing)"), "{plain}");
    assert!(plain.contains("(no subject)"), "{plain}");
    assert!(plain.contains("(empty body)"), "{plain}");
}

#[test]
fn render_gmail_draft_card_wraps_attachments_and_shows_complete_long_body() {
    let body = (1..=30)
        .map(|index| format!("body line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let msg = gmail_draft_message(
        "Draft created successfully.\nDraft ID: draft_long",
        serde_json::json!({
            "action": "draft",
            "to": "a-very-long-recipient-address@example.com",
            "subject": "A subject that should wrap cleanly in a narrow transcript",
            "body": body,
            "attachments": [
                "/tmp/a-very-long-quarterly-report-filename.pdf",
                "/tmp/notes.txt"
            ]
        }),
    );

    let lines = render_tool_message(&msg, 48, crate::config::DiffDisplayMode::Off);
    let rendered = lines.iter().map(extract_line_text).collect::<Vec<_>>();
    let plain = rendered.join("\n");
    let compact = without_whitespace(&plain.replace('│', ""));

    assert!(
        compact.contains("To:a-very-long-recipient-address@example.com"),
        "{plain}"
    );
    assert!(
        compact.contains("Subject:Asubjectthatshouldwrapcleanlyinanarrowtranscript"),
        "{plain}"
    );
    assert!(
        compact
            .contains("Attachments:/tmp/a-very-long-quarterly-report-filename.pdf,/tmp/notes.txt"),
        "{plain}"
    );
    assert!(plain.contains("body line 18"), "{plain}");
    assert!(plain.contains("body line 19"), "{plain}");
    assert!(plain.contains("body line 30"), "{plain}");
    assert!(
        !plain.contains("more lines"),
        "body must not be truncated: {plain}"
    );
    assert!(
        lines.iter().all(|line| line.width() <= 47),
        "draft card exceeded row width: {rendered:?}"
    );
}

#[test]
fn render_gmail_draft_card_preserves_html_like_body_text() {
    let msg = gmail_draft_message(
        "Draft created successfully.\nDraft ID: draft_html",
        serde_json::json!({
            "action": "draft",
            "to": "web@example.com",
            "subject": "HTML-ish content",
            "body": "<p>Hello <strong>team</strong></p>"
        }),
    );

    let plain = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        plain.contains("<p>Hello <strong>team</strong></p>"),
        "{plain}"
    );
}

#[test]
fn render_batch_tool_message_shows_nested_gmail_draft_card() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] gmail ---\nDraft created successfully.\nDraft ID: nested_123\nTo: nested@example.com\nSubject: Nested\n\nCompleted: 1 succeeded, 0 failed".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_batch_gmail".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [{
                    "tool": "gmail",
                    "parameters": {
                        "action": "draft",
                        "to": "nested@example.com",
                        "subject": "Nested",
                        "body": "Created inside a batch"
                    }
                }]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        plain.contains("Gmail draft created · nested_123"),
        "{plain}"
    );
    assert!(plain.contains("Created inside a batch"), "{plain}");
}

#[test]
fn render_batch_tool_message_shows_flat_and_nested_subcall_intents() {
    crate::tui::ui::tools_ui::tests_tool_call_details_override::set(true);
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] read ---\nflat output\n\n--- [2] read ---\nnested output\n\nCompleted: 2 succeeded, 0 failed".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_batch_intents".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [
                    {
                        "tool": "read",
                        "intent": "Inspect flat batch input",
                        "file_path": "src/flat.rs"
                    },
                    {
                        "tool": "read",
                        "parameters": {
                            "intent": "Inspect nested batch input",
                            "file_path": "src/nested.rs"
                        }
                    }
                ]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let plain = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off)
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        plain.contains("read · Inspect flat batch input ·"),
        "{plain}"
    );
    assert!(
        plain.contains("read · Inspect nested batch input ·"),
        "{plain}"
    );
    assert!(plain.contains("flat.rs"), "{plain}");
    assert!(plain.contains("nested.rs"), "{plain}");
    crate::tui::ui::tools_ui::tests_tool_call_details_override::set(false);
}

fn discovery_message(content: &str, input: serde_json::Value) -> DisplayMessage {
    DisplayMessage {
        role: "tool".to_string(),
        content: content.to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_discovery".to_string(),
            name: "integration_tools".to_string(),
            input,
            intent: None,
            thought_signature: None,
        }),
    }
}

#[test]
fn render_tool_message_shows_discovery_browse_results_and_rationale() {
    let msg = discovery_message(
        "Discoverable tools in 'payments' (Jcode tool directory; recommendations must be based only on fit; details: https://jcode.sh/discovery-tools):\n\n- agentcard: prepaid virtual Visa cards for AI agents (https://agentcard.sh/?via=jcode-discovery)\n\nSearch request ID: `11111111-2222-4333-8444-555555555555`",
        serde_json::json!({
            "action": "search",
            "category": "payments",
            "query": "manage Stripe sandbox products and recurring prices",
            "reason": "the task needs test-mode catalog administration through scoped agent access"
        }),
    );
    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(plain.contains("agentcard"), "{plain}");
    assert!(
        !plain.contains("1 integration"),
        "single-result browse shows only the entry name: {plain}"
    );
    assert!(
        !plain.contains("why:"),
        "browse results stay to a single line without rationale: {plain}"
    );
    assert!(
        !plain.contains("prepaid virtual Visa cards"),
        "browse results must not render descriptions: {plain}"
    );
    assert!(
        !plain.contains("agentcard.sh"),
        "browse results must not render URLs: {plain}"
    );
    assert!(
        !plain.contains("Listings are vetted"),
        "discovery results must not render the disclosure notice: {plain}"
    );
    assert!(!plain.contains("sponsored result"), "{plain}");
    assert!(
        lines.len() <= 8,
        "compact discovery details used {} lines: {plain}",
        lines.len()
    );
    assert!(
        !plain.contains("\n\n"),
        "compact details contain a blank row: {plain}"
    );
    assert!(
        !plain
            .chars()
            .any(|ch| matches!(ch, '╭' | '╮' | '╰' | '╯' | '│')),
        "discovery details must remain borderless: {plain}"
    );
    assert!(
        !plain.contains("11111111-2222"),
        "request IDs stay model-only: {plain}"
    );
}

#[test]
fn batched_discovery_renders_without_disclosure_notice() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "--- [1] integration_tools ---\nAvailable integrations in 'payments' (Jcode tool directory; recommendations must be based only on fit; details: https://jcode.sh/discovery-tools):\n\n- agentcard: prepaid virtual Visa cards for AI agents (https://agentcard.sh/?via=jcode-discovery)\n\nSearch request ID: `11111111-2222-4333-8444-555555555555`\n\nCompleted: 1 succeeded, 0 failed".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_batch_discovery".to_string(),
            name: "batch".to_string(),
            input: serde_json::json!({
                "tool_calls": [{
                    "tool": "integration_tools",
                    "parameters": {
                        "action": "search",
                        "category": "payments",
                        "query": "issue a capped virtual card",
                        "reason": "the task requires a payment instrument with a hard limit"
                    }
                }]
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("agentcard"), "{plain}");
    assert!(
        !plain.contains("1 integration"),
        "single-result browse shows only the entry name: {plain}"
    );
    assert!(
        !plain.contains("Listings are vetted"),
        "batched discovery must not render the disclosure notice: {plain}"
    );
    assert!(
        !plain
            .chars()
            .any(|ch| matches!(ch, '╭' | '╮' | '╰' | '╯' | '│')),
        "batched discovery details must remain borderless: {plain}"
    );
}

#[test]
fn render_tool_message_shows_selected_discovery_setup() {
    let msg = discovery_message(
        "Selected 'agentcard' from 'payments' (Jcode tool directory; selection must be based only on fit; details: https://jcode.sh/discovery-tools):\n\nagentcard: prepaid virtual Visa cards for AI agents (https://agentcard.sh/?via=jcode-discovery)\n\nSetup: Run `npx -y agentcard-mcp@1.2.3`, then connect the resulting MCP server.\n\nConsequential actions (signups, spending) must note the partnership in the confirmation shown to the user.",
        serde_json::json!({
            "action": "select",
            "category": "payments",
            "tool": "agentcard",
            "query": "create a capped virtual card for an online purchase",
            "reason": "selected because capped cards fit the purchase constraints better than alternatives"
        }),
    );
    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(plain.contains("selected agentcard"), "{plain}");
    assert!(!plain.contains("sponsored"), "{plain}");
    assert!(
        plain.contains("details: prepaid virtual Visa cards"),
        "{plain}"
    );
    assert!(plain.contains("https://agentcard.sh"), "{plain}");
    assert!(plain.contains("setup:"), "{plain}");
    assert!(plain.contains("agentcard-mcp@1.2.3"), "{plain}");
    assert!(
        !plain.contains("Listings are vetted"),
        "discovery results must not render the disclosure notice: {plain}"
    );
}

#[test]
fn render_tool_message_does_not_duplicate_selected_when_tool_is_missing() {
    let msg = discovery_message(
        "Selection recorded.",
        serde_json::json!({
            "action": "select",
            "category": "web-search",
            "query": "find current public estimates"
        }),
    );
    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(plain.contains("selected tool"), "{plain}");
    assert!(!plain.contains("selected selected tool"), "{plain}");
}

#[test]
fn render_tool_message_marks_off_catalog_selection_without_fake_details() {
    let msg = discovery_message(
        "Selected off-catalog product 'firecrawl' for 'web-data'.\n\nSelection recorded as demand data. Jcode does not list or partner with this product, so no provider information, recommendation, or setup instructions are provided.",
        serde_json::json!({
            "action": "select",
            "category": "web-data",
            "tool": "firecrawl",
            "query": "crawl a documentation site and extract structured markdown",
            "reason": "the user explicitly requested Firecrawl instead of the catalog listing"
        }),
    );
    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(plain.contains("selected off-catalog firecrawl"), "{plain}");
    assert!(
        plain.contains("why: the user explicitly requested"),
        "{plain}"
    );
    assert!(!plain.contains("details:"), "{plain}");
    assert!(!plain.contains("setup:"), "{plain}");
}

#[test]
fn render_tool_message_shows_catalog_suggestion_receipt_and_trust_line() {
    let msg = discovery_message(
        "Catalog suggestion submitted.\n\nSuggestion ID: aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee\nCategory: payments\nKind: known_product\nCapability: manage Stripe sandbox products\nCatalog gap: no matching catalog entry\nProduct: Stripe sandbox MCP\nPublic URL: https://example.com/stripe-mcp\n\nStatus: received for Jcode maintainer review. Suggestions are not sent to partners. This does not mean Jcode has partnered with the tool or that it is approved or available.",
        serde_json::json!({
            "action": "suggest",
            "category": "payments",
            "query": "manage Stripe sandbox products and recurring prices through scoped agent access",
            "reason": "the listed payment tool only provides cards and cannot administer Stripe test data",
            "suggestion_kind": "known_product",
            "product_name": "Stripe sandbox MCP",
            "product_url": "https://example.com/stripe-mcp",
            "gap_evidence": "Agentcard handles cards rather than Stripe test-mode objects.",
            "requirements": [
                "Scoped authentication without exposing a secret key",
                "Create recurring prices in test mode"
            ],
            "prior_request_id": "11111111-2222-4333-8444-555555555555"
        }),
    );
    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Off);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(plain.contains("suggestion sent"), "{plain}");
    assert!(
        plain.contains("Known product · Stripe sandbox MCP"),
        "{plain}"
    );
    assert!(plain.contains("gap: the listed payment tool"), "{plain}");
    assert!(plain.contains("needs:"), "{plain}");
    assert!(plain.contains("Jcode maintainers only"), "{plain}");
    assert!(plain.contains("not approval or availability"), "{plain}");
    assert!(
        !plain.contains("11111111-2222"),
        "prior request ID must stay hidden: {plain}"
    );
}

#[test]
fn discovery_cards_wrap_within_narrow_transcript_width() {
    let msg = discovery_message(
        "Catalog suggestion submitted.\n\nStatus: received for Jcode maintainer review.",
        serde_json::json!({
            "action": "suggest",
            "category": "cloud-infrastructure",
            "query": "a deliberately long capability description that must wrap cleanly in a narrow terminal",
            "reason": "the current catalog entries do not satisfy several detailed infrastructure constraints",
            "suggestion_kind": "capability_gap",
            "requirements": ["A long requirement that also needs reliable narrow-width wrapping"]
        }),
    );
    let lines = render_tool_message(&msg, 48, crate::config::DiffDisplayMode::Off);
    assert!(
        lines.iter().all(|line| line.width() <= 47),
        "discovery card exceeded width: {:?}",
        lines.iter().map(extract_line_text).collect::<Vec<_>>()
    );
}

#[test]
fn render_tool_message_colors_high_token_badge() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "x".repeat(48_000),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: None,
        tool_data: Some(crate::message::ToolCall {
            id: "call_3".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"file_path": "src/main.rs"}),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 120, crate::config::DiffDisplayMode::Off);
    let badge_span = lines[0]
        .spans
        .iter()
        .find(|span| span.content.contains("12k tok"))
        .expect("missing token badge");

    assert_eq!(badge_span.style.fg, Some(rgb(224, 118, 118)));
}

#[test]
fn render_tool_message_shows_inline_diff_for_pascal_case_multiedit() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "Edited demo.txt\n\nApplied:\n  ✓ Edit 1: replaced 1 occurrence\n\nTotal: 1 applied, 0 failed\n"
            .to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("demo.txt".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_multiedit_pascal".to_string(),
            name: "MultiEdit".to_string(),
            input: serde_json::json!({
                "file_path": "demo.txt",
                "edits": [
                    {"old_string": "old line\n", "new_string": "new line\n"}
                ]
            }),
            intent: None, thought_signature: None, }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Inline);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("┌─ diff"), "plain={plain}");
    assert!(plain.contains("old line"), "plain={plain}");
    assert!(plain.contains("new line"), "plain={plain}");
}

#[test]
fn render_tool_message_labels_single_file_apply_patch_diff() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "✓ src/example.rs: modified (1 hunks)".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("src/example.rs".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_apply_patch_single".to_string(),
            name: "apply_patch".to_string(),
            input: serde_json::json!({
                "intent": "Update example behavior",
                "patch_text": "*** Begin Patch\n*** Update File: src/example.rs\n@@\n-old_value\n+new_value\n*** End Patch\n"
            }),
            intent: Some("Update example behavior".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Inline);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("┌─ diff · src/example.rs"), "plain={plain}");
    assert!(plain.contains("old_value"), "plain={plain}");
    assert!(plain.contains("new_value"), "plain={plain}");
}

#[test]
fn render_tool_message_preserves_multi_file_apply_patch_boundaries() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "✓ a.txt: modified (1 hunks)\n1- old a\n1+ new a\n✓ b.txt: modified (1 hunks)\n1- old b\n1+ new b\n".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("2 files".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_apply_patch_multi".to_string(),
            name: "apply_patch".to_string(),
            input: serde_json::json!({
                "intent": "Update both examples",
                "patch_text": "*** Begin Patch\n*** Update File: a.txt\n@@\n-old a\n+new a\n*** Update File: b.txt\n@@\n-old b\n+new b\n*** End Patch\n"
            }),
            intent: Some("Update both examples".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Inline);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    let a_header = plain.find("┌─ diff · a.txt").expect("missing a.txt header");
    let old_a = plain.find("old a").expect("missing a.txt deletion");
    let new_a = plain.find("new a").expect("missing a.txt addition");
    let b_header = plain
        .find("├─ diff · b.txt")
        .expect("missing b.txt boundary");
    let old_b = plain.find("old b").expect("missing b.txt deletion");
    let new_b = plain.find("new b").expect("missing b.txt addition");
    assert!(
        a_header < old_a && old_a < new_a && new_a < b_header,
        "plain={plain}"
    );
    assert!(b_header < old_b && old_b < new_b, "plain={plain}");
}

#[test]
fn render_tool_message_shows_numbered_write_result_diff_after_input_compaction() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "Created /tmp/head-to-head.html (2 lines):\n1+ <!doctype html>\n2+ <html lang=\"en\">\n..."
            .to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("/tmp/head-to-head.html".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_write_compacted".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({"file_path": "/tmp/head-to-head.html"}),
            intent: Some("Create an honest data-driven benchmark comparison page".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Inline);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        lines[0].spans.iter().any(|span| span.content == "+2"),
        "plain={plain}"
    );
    assert!(plain.contains("┌─ diff"), "plain={plain}");
    assert!(plain.contains("<!doctype html>"), "plain={plain}");
    assert!(plain.contains("<html lang=\"en\">"), "plain={plain}");
}

#[test]
fn render_tool_message_never_draws_an_empty_edit_diff_frame() {
    for (name, content) in [
        ("write", "Created empty.txt (0 lines):\n"),
        ("edit", "Edited demo.txt: replaced 1 occurrence(s)"),
        (
            "multiedit",
            "Edited demo.txt\n\nTotal: 1 applied, 0 failed\n",
        ),
        ("patch", "Patch applied successfully"),
        ("apply_patch", "✓ demo.txt: modified (1 hunks)"),
    ] {
        let msg = DisplayMessage {
            role: "tool".to_string(),
            content: content.to_string(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: Some("demo.txt".to_string()),
            tool_data: Some(crate::message::ToolCall {
                id: format!("call_{name}_compacted"),
                name: name.to_string(),
                input: serde_json::json!({"file_path": "demo.txt"}),
                intent: None,
                thought_signature: None,
            }),
        };

        let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Inline);
        let plain = lines
            .iter()
            .map(extract_line_text)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!plain.contains("┌─ diff"), "tool={name}, plain={plain}");
        assert!(!plain.contains("(+0 -0)"), "tool={name}, plain={plain}");
    }
}

#[test]
fn render_tool_message_marks_failed_apply_patch_without_empty_diff() {
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content:
            "[apply_patch] ✗ /tmp/main.rs: Failed to find expected lines in /tmp/main.rs:\nfn missing() {}"
                .to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("/tmp/main.rs".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_apply_patch_failed".to_string(),
            name: "apply_patch".to_string(),
            input: serde_json::json!({"file_path": "/tmp/main.rs"}),
            intent: Some("Replace the benchmark placeholder".to_string()),
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 100, crate::config::DiffDisplayMode::Inline);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        plain.trim_start().starts_with("✗ apply_patch"),
        "plain={plain}"
    );
    assert!(!plain.contains("┌─ diff"), "plain={plain}");
    assert!(!plain.contains("(+0 -0)"), "plain={plain}");
}

#[test]
fn render_tool_message_inline_mode_truncates_large_diffs() {
    let old = (1..=7)
        .map(|i| format!("old line {i}\n"))
        .collect::<String>();
    let new = (1..=7)
        .map(|i| format!("new line {i} suffix_{i}_abcdefghijklmnopqrstuvwxyz0123456789\n"))
        .collect::<String>();
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "Edited demo.txt".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("demo.txt".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_edit_inline_truncated".to_string(),
            name: "edit".to_string(),
            input: serde_json::json!({
                "file_path": "demo.txt",
                "old_string": old,
                "new_string": new,
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 40, crate::config::DiffDisplayMode::Inline);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(plain.contains("... 2 more changes ..."), "plain={plain}");
    assert!(plain.contains("old line 3"), "plain={plain}");
    assert!(!plain.contains("old line 7"), "plain={plain}");
    assert!(
        !plain.contains("new line 1 suffix_1_abcdefghijklmnopqrstuvwxyz0123456789"),
        "plain={plain}"
    );
    assert!(plain.contains("suffix_2_abcdefghijklm…"), "plain={plain}");
}

#[test]
fn render_tool_message_full_inline_mode_shows_full_diff() {
    let old = (1..=7)
        .map(|i| format!("old line {i}\n"))
        .collect::<String>();
    let new = (1..=7)
        .map(|i| format!("new line {i} suffix_{i}_abcdefghijklmnopqrstuvwxyz0123456789\n"))
        .collect::<String>();
    let msg = DisplayMessage {
        role: "tool".to_string(),
        content: "Edited demo.txt".to_string(),
        tool_calls: Vec::new(),
        duration_secs: None,
        title: Some("demo.txt".to_string()),
        tool_data: Some(crate::message::ToolCall {
            id: "call_edit_inline_full".to_string(),
            name: "edit".to_string(),
            input: serde_json::json!({
                "file_path": "demo.txt",
                "old_string": old,
                "new_string": new,
            }),
            intent: None,
            thought_signature: None,
        }),
    };

    let lines = render_tool_message(&msg, 40, crate::config::DiffDisplayMode::FullInline);
    let plain = lines
        .iter()
        .map(extract_line_text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!plain.contains("more changes"), "plain={plain}");
    assert!(plain.contains("old line 4"), "plain={plain}");
    assert!(
        plain.contains("new line 4 suffix_4_abcdefghijklmnopqrstuvwxyz0123456789"),
        "plain={plain}"
    );
    assert!(!plain.contains('…'), "plain={plain}");
}
