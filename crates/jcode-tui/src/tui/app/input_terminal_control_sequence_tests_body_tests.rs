use super::strip_terminal_control_sequences;

/// Remnants of terminal reports must never reach the composer (#540).
#[test]
fn strips_escape_and_bare_report_remnants() {
    for (input, expected) in [
        // Full mouse report, and the bare tail left by a torn read.
        ("\x1b[<65;50;24M", ""),
        ("[<65;50;24M", ""),
        ("hi[<65;50;24Mthere", "hithere"),
        ("[<65;50;24m", ""),
        // Bracketed paste markers and cursor/focus reports.
        ("[200~", ""),
        ("[201~", ""),
        ("[12;40R", ""),
        ("[1I", ""),
        ("[1O", ""),
        // 8-bit CSI introducer.
        ("\u{9b}[<65;50;24M", ""),
        // Stray C0 controls, but tabs and newlines survive.
        ("a\x07b", "ab"),
        ("a\tb\nc", "a\tb\nc"),
        // Truncated escape with no final byte: drop the remnant.
        ("\x1b[<65;5", ""),
    ] {
        assert_eq!(
            strip_terminal_control_sequences(input),
            expected,
            "input {input:?} should sanitize to {expected:?}"
        );
    }
}

/// The guard must not eat text a user actually typed. Being too aggressive
/// here is worse than missing a remnant.
#[test]
fn preserves_ordinary_bracketed_text() {
    for input in [
        "array[0]",
        "list[1] = list[2]",
        "[TODO] fix this",
        "see docs[1] and notes[2]",
        "fn f(v: Vec<u8>) -> [u8; 4]",
        "a[b]c",
        "[]",
        "[",
        "]",
        "[abc]",
        "[1]",
        "[12;40]",
        "plain text with no brackets",
        "emoji 🎉 and accents café",
        "match x { [a, b] => a + b }",
    ] {
        assert_eq!(
            strip_terminal_control_sequences(input),
            input,
            "input {input:?} must be preserved verbatim"
        );
    }
}

/// Non-suspicious text must not be reallocated.
#[test]
fn borrows_when_nothing_to_strip() {
    assert!(matches!(
        strip_terminal_control_sequences("array[0] = 1"),
        std::borrow::Cow::Borrowed(_)
    ));
}
