//! Focused cache equivalence regression tests.
use super::*;

/// Prefix-reuse must produce a body byte-identical to a fresh full build when
/// the tail message is edited in place (e.g. a streaming tool result is
/// finalized). This exercises truncate + re-append on the changed tail.
#[test]
fn test_prefix_reuse_tail_edit_matches_full_build() {
    let width = 64;
    let base_state = TestState {
        display_messages: vec![
            DisplayMessage::user("first prompt"),
            DisplayMessage::assistant(
                "a fairly long answer that wraps across the width boundary here",
            ),
            DisplayMessage::user("second prompt"),
            DisplayMessage::assistant("partial"),
        ],
        messages_version: 1,
        ..Default::default()
    };
    // Tail (last assistant) edited in place; prefix of 3 messages unchanged.
    let edited_state = TestState {
        display_messages: vec![
            DisplayMessage::user("first prompt"),
            DisplayMessage::assistant(
                "a fairly long answer that wraps across the width boundary here",
            ),
            DisplayMessage::user("second prompt"),
            DisplayMessage::assistant(
                "partial answer is now complete and considerably longer than before",
            ),
        ],
        messages_version: 2,
        ..Default::default()
    };

    let base = Arc::new(super::super::prepare::prepare_body(
        &base_state,
        width,
        false,
    ));
    let k =
        super::super::prepare::matching_prefix_len(base.as_ref(), &edited_state.display_messages);
    assert_eq!(k, 3, "only the last message changed");

    let mut reuse = base;
    super::super::prepare::truncate_prepared_to_boundary(Arc::make_mut(&mut reuse), k);
    let reuse = super::super::prepare::prepare_body_incremental(&edited_state, width, reuse, k);
    let full = super::super::prepare::prepare_body(&edited_state, width, false);
    assert_prepared_equivalent(&reuse, &full, &edited_state.display_messages, "tail_edit");
}

/// Pure append still matches a full build (k == prev_count path).
#[test]
fn test_prefix_reuse_append_matches_full_build() {
    let width = 50;
    let base_state = TestState {
        display_messages: vec![
            DisplayMessage::user("hello there"),
            DisplayMessage::assistant("hi, how can I help you today with this task"),
        ],
        messages_version: 1,
        ..Default::default()
    };
    let grown_state = TestState {
        display_messages: vec![
            DisplayMessage::user("hello there"),
            DisplayMessage::assistant("hi, how can I help you today with this task"),
            DisplayMessage::user("another question that is fairly long and wraps too"),
            DisplayMessage::assistant("sure, here is the answer to your second question"),
        ],
        messages_version: 2,
        ..Default::default()
    };

    let base = Arc::new(super::super::prepare::prepare_body(
        &base_state,
        width,
        false,
    ));
    let k =
        super::super::prepare::matching_prefix_len(base.as_ref(), &grown_state.display_messages);
    assert_eq!(k, 2);
    let reuse = super::super::prepare::prepare_body_incremental(&grown_state, width, base, k);
    let full = super::super::prepare::prepare_body(&grown_state, width, false);
    assert_prepared_equivalent(&reuse, &full, &grown_state.display_messages, "append");
}

/// Truncation (transcript shrank, e.g. a rewind) reuses the surviving prefix
/// and matches a full build.
#[test]
fn test_prefix_reuse_truncation_matches_full_build() {
    let width = 48;
    let long_state = TestState {
        display_messages: vec![
            DisplayMessage::user("q1 that is reasonably long to force wrapping in body"),
            DisplayMessage::assistant("answer one spanning multiple wrapped lines for sure"),
            DisplayMessage::user("q2 also long enough to wrap across the configured width"),
            DisplayMessage::assistant("answer two also spanning several wrapped output lines"),
        ],
        messages_version: 1,
        ..Default::default()
    };
    let short_state = TestState {
        display_messages: vec![
            DisplayMessage::user("q1 that is reasonably long to force wrapping in body"),
            DisplayMessage::assistant("answer one spanning multiple wrapped lines for sure"),
        ],
        messages_version: 2,
        ..Default::default()
    };

    let base = Arc::new(super::super::prepare::prepare_body(
        &long_state,
        width,
        false,
    ));
    let k =
        super::super::prepare::matching_prefix_len(base.as_ref(), &short_state.display_messages);
    assert_eq!(k, 2);
    let mut reuse = base;
    super::super::prepare::truncate_prepared_to_boundary(Arc::make_mut(&mut reuse), k);
    // After truncation alone (no new tail to append) it must already match.
    let full = super::super::prepare::prepare_body(&short_state, width, false);
    assert_prepared_equivalent(&reuse, &full, &short_state.display_messages, "truncation");
}

/// Regression for #735 at the TUI layer.
///
/// `prepare_body` runs on the draw path, the same thread that services
/// keystrokes and interrupts. When display math rendered synchronously, a
/// stalled TeX toolchain (the reporter's `latex` was regenerating its format
/// files) blocked this call for the per-command timeout on every uncached
/// formula. The user-visible result was a client that could not act on Esc:
/// the server cancelled the turn in milliseconds while the TUI sat
/// unresponsive, so "Interrupting" lingered and then vanished.
///
/// Two things must hold: preparing a transcript full of uncached formulas
/// stays fast even when the toolchain never returns, and the resulting body
/// carries the pending stamp, without which the placeholder would never be
/// replaced by the finished image.
#[cfg(unix)]
#[test]
fn test_prepare_body_with_math_never_blocks_on_a_stalled_tex_toolchain() {
    // Earlier tests can cache a missing toolchain. Exercise the real probe and
    // worker in a fresh process, not that process-global cached result.
    if std::env::var_os("JCODE_STALLED_TEX_TEST_CHILD").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tui::ui::tests::basic::cache_equivalence::test_prepare_body_with_math_never_blocks_on_a_stalled_tex_toolchain",
                "--nocapture",
            ])
            .env("JCODE_STALLED_TEX_TEST_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success(), "isolated stalled TeX regression failed");
        return;
    }
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let stub = dir.path().join("hanging-tex");
    let mut file = std::fs::File::create(&stub).unwrap();
    file.write_all(b"#!/bin/sh\nsleep 120\nexit 1\n").unwrap();
    drop(file);
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: read only by the render and worker threads spawned below.
    unsafe {
        std::env::set_var("JCODE_LATEX_COMMAND", &stub);
        std::env::set_var("JCODE_DVIPNG_COMMAND", &stub);
        std::env::set_var("JCODE_PDFLATEX_COMMAND", &stub);
        std::env::set_var("JCODE_PDFTOCAIRO_COMMAND", &stub);
    }

    // Unique formulas so no previously cached artifact short-circuits this.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let math: String = (0..8)
        .map(|i| format!("$$w_{{{nonce}_{i}}} = \\frac{{{i}}}{{\\sqrt{{2}}}}$$\n\n"))
        .collect();
    let state = TestState {
        display_messages: vec![
            DisplayMessage::user("derive it"),
            DisplayMessage::assistant(&math),
        ],
        messages_version: 1,
        ..Default::default()
    };

    // Image mode only engages when the terminal advertises graphics, as the
    // reporter's terminal did; a test process otherwise skips the LaTeX path.
    let started = std::time::Instant::now();
    let prepared = crate::tui::mermaid::with_image_protocol_override(Some(true), || {
        super::super::prepare::prepare_body(&state, 90, false)
    });
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(3),
        "prepare_body blocked the draw path for {elapsed:?} with a stalled TeX toolchain; \
         keystrokes and interrupts cannot be serviced while it does (#735)"
    );
    assert!(
        prepared.mermaid_pending_epoch.is_some(),
        "deferred formulas must stamp the prepared body, or the completed render \
         never invalidates the cache and the placeholder stays on screen forever"
    );

    unsafe {
        std::env::remove_var("JCODE_LATEX_COMMAND");
        std::env::remove_var("JCODE_DVIPNG_COMMAND");
        std::env::remove_var("JCODE_PDFLATEX_COMMAND");
        std::env::remove_var("JCODE_PDFTOCAIRO_COMMAND");
    }
}

#[test]
fn test_cache_miss_footer_attaches_in_full_and_incremental_layout() {
    let base_state = TestState {
        display_messages: vec![
            DisplayMessage::user("question"),
            DisplayMessage::assistant("answer"),
        ],
        messages_version: 1,
        ..Default::default()
    };
    let mut messages = base_state.display_messages.clone();
    messages.push(DisplayMessage::system(
        "ℹ️ KV cache miss [low read]: ~4K tokens resent",
    ));
    messages.push(DisplayMessage::user("next question"));
    let grown_state = TestState {
        display_messages: messages,
        messages_version: 2,
        ..Default::default()
    };
    for width in [20, 80] {
        let base = Arc::new(super::super::prepare::prepare_body(
            &base_state,
            width,
            false,
        ));
        let incremental =
            super::super::prepare::prepare_body_incremental(&grown_state, width, base, 2);
        let full = super::super::prepare::prepare_body(&grown_state, width, false);
        let plain = |prepared: &super::super::PreparedMessages| {
            prepared
                .wrapped_lines
                .iter()
                .map(super::super::line_plain_text)
                .collect::<Vec<_>>()
        };
        let full_lines = plain(&full);
        assert_eq!(plain(&incremental), full_lines);
        let footer = full_lines
            .iter()
            .position(|line| line.starts_with("╰ "))
            .unwrap();
        assert_eq!(full_lines[footer - 1].trim(), "answer");
        assert!(full_lines[footer + 1].trim().is_empty());
    }
}
