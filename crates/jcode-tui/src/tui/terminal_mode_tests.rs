use super::reapply_terminal_modes_to;

#[test]
fn reapply_does_not_request_another_focus_report() {
    for mouse_capture in [false, true] {
        for keyboard_enhanced in [false, true] {
            let mut output = Vec::new();
            reapply_terminal_modes_to(&mut output, mouse_capture, keyboard_enhanced).unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.contains("\x1b[?2004h"));
            assert!(
                !output.contains("\x1b[?1004h"),
                "rearming focus reports makes Ghostty reply with another FocusGained"
            );
            assert!(!output.contains("\x1b[?1004l"), "keep reporting enabled");
        }
    }
}

#[test]
fn reapply_omits_mouse_sequences_when_capture_is_disabled() {
    let mut output = Vec::new();
    reapply_terminal_modes_to(&mut output, false, true).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.starts_with("\x1b[?2004h"));
    assert!(!output.contains("\x1b[?1000h"));
    assert!(output.contains("\x1b[="));
}

#[test]
fn reapply_emits_configured_idempotent_modes_without_keyboard_push() {
    let mut output = Vec::new();
    reapply_terminal_modes_to(&mut output, true, true).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("\x1b[?2004h"));
    assert!(!output.contains("\x1b[?1004h"));
    assert!(output.contains("\x1b[?1000h"));
    assert!(output.contains("\x1b[="), "must set Kitty keyboard flags");
    assert!(
        !output.contains("\x1b[>"),
        "must not push the Kitty keyboard stack"
    );
}
