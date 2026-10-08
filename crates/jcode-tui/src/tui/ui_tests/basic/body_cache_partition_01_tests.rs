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
    // The toolchain probe is process-global. Exercise a fresh probe rather
    // than a missing-toolchain result cached by an earlier test.
    if std::env::var_os("JCODE_STALLED_TEX_TEST_CHILD").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tui::ui::tests::basic::test_prepare_body_with_math_never_blocks_on_a_stalled_tex_toolchain",
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
        super::prepare::prepare_body(&state, 90, false)
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
