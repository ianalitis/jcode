//! Tolerance tests: the agentgrep failure classes measured from the daemon logs and
//! the session journals, each of which used to cost a turn.
//!
//! Split from `agentgrep_tests.rs` so neither file passes the 1200-line test-size
//! ratchet, and kept next to it because they test the same tool through the same
//! helpers.

use super::test_support::test_ctx;
use super::*;
use std::fs;

// ---------------------------------------------------------------------------
// Recurring failure classes, measured from the session journals: a query that
// cannot compile as a regex, a repeated trace DSL key, and a call that names a
// file with no query and no mode. Each one used to fail the turn; each now runs
// with the substitution named in the output.
// ---------------------------------------------------------------------------

fn grep_args_with(query: &str, regex: bool) -> GrepArgs {
    GrepArgs {
        query: query.to_string(),
        regex,
        file_type: None,
        json: false,
        paths_only: false,
        hidden: false,
        no_ignore: false,
        path: None,
        glob: None,
        no_follow: false,
    }
}

#[test]
fn a_query_that_is_not_a_valid_regex_is_searched_literally() {
    // The measured case: Rust source text passed with regex=true. `fn(` has an
    // unclosed group, so upstream reports "invalid regex" and the turn is lost.
    let mut args = grep_args_with("comm_session_stop_tests|fn(", true);
    let note = args::degrade_uncompilable_regex(&mut args).expect("an invalid regex is reported");

    assert!(!args.regex, "the search must fall back to literal");
    assert_eq!(
        args.query, "comm_session_stop_tests|fn(",
        "the text itself is unchanged"
    );
    assert!(note.contains("searched literally instead"), "{note}");
    assert!(note.starts_with("note:"), "{note}");
    assert!(
        note.contains("unclosed group"),
        "the reason belongs in the note: {note}"
    );
    assert!(
        !note.contains('\n'),
        "the note is one line, so it cannot be mistaken for the search output: {note}"
    );
}

#[test]
fn a_valid_regex_is_left_alone() {
    let mut args = grep_args_with("fn\\(", true);
    assert!(args::degrade_uncompilable_regex(&mut args).is_none());
    assert!(args.regex, "a compilable regex must keep regex semantics");
}

#[test]
fn a_literal_search_is_never_second_guessed() {
    // regex=false with text that would not compile is the normal literal path.
    let mut args = grep_args_with("fn(", false);
    assert!(args::degrade_uncompilable_regex(&mut args).is_none());
    assert!(!args.regex);
}

#[test]
fn repeated_trace_keys_keep_the_first_and_report_the_rest() {
    let terms = vec![
        "subject:auth_status".to_string(),
        "relation:rendered".to_string(),
        "subject:other_subject".to_string(),
        "support".to_string(),
        "support".to_string(),
    ];
    let (kept, note) = args::dedupe_smart_dsl_keys(&terms);

    assert_eq!(
        kept,
        vec![
            "subject:auth_status".to_string(),
            "relation:rendered".to_string(),
            "support".to_string(),
            "support".to_string(),
        ],
        "the first value of each key wins and bare terms pass through untouched"
    );
    let note = note.expect("the dropped term is reported");
    assert!(note.contains("subject:other_subject"), "{note}");
}

#[test]
fn unique_trace_keys_produce_no_note() {
    let terms = vec!["subject:auth".to_string(), "relation:rendered".to_string()];
    let (kept, note) = args::dedupe_smart_dsl_keys(&terms);
    assert_eq!(kept, terms);
    assert!(note.is_none(), "nothing to report when nothing was dropped");
}

#[test]
fn a_file_with_no_query_and_no_mode_runs_as_outline() {
    // The measured case: `file_path: starter/CRAFT-CONTRACT.md` with an intent to
    // outline and no `mode`, which defaulted to grep and failed on the missing query.
    let params: AgentGrepInput = serde_json::from_value(json!({
        "file_path": "starter/CRAFT-CONTRACT.md",
        "intent": "outline the craft contract"
    }))
    .expect("the call deserializes");

    let (mode, note) =
        args::infer_outline_for_file_only_call(&params).expect("outline is inferred");
    assert_eq!(mode, "outline");
    assert!(note.contains("ran as outline"), "{note}");
}

#[test]
fn mode_inference_leaves_real_searches_alone() {
    let with_query: AgentGrepInput = serde_json::from_value(json!({
        "query": "auth_status",
        "file_path": "src/lib.rs"
    }))
    .expect("deserializes");
    assert!(
        args::infer_outline_for_file_only_call(&with_query).is_none(),
        "a query means grep, whatever else was passed"
    );

    let directory: AgentGrepInput = serde_json::from_value(json!({
        "path": "docs/decisions"
    }))
    .expect("deserializes");
    assert!(
        args::infer_outline_for_file_only_call(&directory).is_none(),
        "a bare directory is not a file to outline"
    );

    let explicit: AgentGrepInput = serde_json::from_value(json!({
        "mode": "find",
        "query": "agentgrep"
    }))
    .expect("deserializes");
    assert!(args::infer_outline_for_file_only_call(&explicit).is_none());
}

#[test]
fn the_missing_query_error_names_the_way_out() {
    let ctx = test_ctx(Path::new("/workspace"));
    let params: AgentGrepInput = serde_json::from_value(json!({"path": "docs"})).expect("parses");

    let error = build_grep_args(&params, &ctx).expect_err("grep needs a query");
    let message = error.to_string();
    assert!(
        message.contains("needs something to search for"),
        "{message}"
    );
    assert!(
        message.contains("pattern"),
        "the alias is worth naming: {message}"
    );
    assert!(
        message.contains("outline"),
        "so is the file-inspection path: {message}"
    );
}

#[test]
fn an_invalid_regex_still_returns_matches_with_a_note() {
    // End to end through the real linked search: the turn that used to fail now
    // returns the matches it was looking for, with the substitution named first.
    let temp = tempfile::tempdir().expect("temp dir");
    // The text to find must contain `fn(` literally: a search for source text is
    // exactly the case where a caller reaches for regex=true and the pattern turns
    // out not to compile.
    fs::write(
        temp.path().join("sample.rs"),
        "fn outer() {\n    let x = call_fn();\n}\n// note: fn( is unbalanced here\n",
    )
    .expect("write sample");
    let ctx = test_ctx(temp.path());
    let params: AgentGrepInput = serde_json::from_value(json!({
        "mode": "grep",
        "query": "fn(",
        "regex": true
    }))
    .expect("deserializes");

    let output = execute_linked_agentgrep(&params, &ctx, None, &[]).expect("the search runs");
    assert!(
        output.output.contains("searched literally instead"),
        "the substitution must be visible: {}",
        output.output
    );
    assert!(
        output.output.contains("sample.rs"),
        "the literal search must still find the text: {}",
        output.output
    );
}
