use super::*;

#[test]
fn renders_common_inline_notation() {
    assert_eq!(render_inline_latex(r"E = mc^2"), "E = mc²");
    assert_eq!(render_inline_latex(r"\alpha + \beta \leq \pi"), "α + β ≤ π");
    assert_eq!(render_inline_latex(r"\alpha ^2 + \beta _1"), "α² + β₁");
    assert_eq!(render_inline_latex(r"x_{10}"), "x₁₀");
    assert_eq!(render_inline_latex(r"x_i^2"), "xᵢ²");
    assert_eq!(render_inline_latex(r"x^2^3"), "x²³");
    assert_eq!(render_inline_latex(r"\frac{a+b}{c}"), "(a+b)⁄c");
    assert_eq!(render_inline_latex(r"\sqrt[3]{x}"), "³√x");
    assert_eq!(render_inline_latex(r"\sin x + \log n"), "sin x + log n");
    assert_eq!(render_inline_latex(r"\sqrt[\sqrt[2]{y}]{x}"), "^(²√y)√x");
}

#[test]
fn renders_fraction_as_display_layout() {
    assert_eq!(
        render_display_latex(r"\frac{x+1}{y}"),
        vec![" x+1 ", "─────", "  y  "]
    );
}

#[test]
fn renders_matrix_with_tall_brackets() {
    assert_eq!(
        render_display_latex(r"\begin{bmatrix}a & b \\ c & d\end{bmatrix}"),
        vec!["⎡ a  b ⎤", "⎣ c  d ⎦"]
    );
}

#[test]
fn preserves_unknown_commands() {
    assert_eq!(render_inline_latex(r"\custom{x}"), r"\customx");
}

#[test]
fn malformed_input_stays_visible_without_panicking() {
    for source in [
        r"\frac{x",
        r"\sqrt[3{x}",
        r"\begin{bmatrix}a & b",
        r"x^{y_{z}",
        r"\left\{x\right",
        "α_{😀",
    ] {
        let inline = render_inline_latex(source);
        let display = render_display_latex(source);
        assert!(
            !inline.is_empty() || !display.is_empty(),
            "malformed source disappeared: {source:?}"
        );
    }
}

#[test]
fn deeply_nested_input_is_bounded_and_does_not_overflow_the_stack() {
    let source = format!("{}x{}", "{".repeat(20_000), "}".repeat(20_000));
    let repeated_scripts = format!("x{}", "^1".repeat(20_000));
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            assert!(render_inline_latex(&source).contains('x'));
            assert!(
                render_display_latex(&source)
                    .iter()
                    .any(|line| line.contains('x'))
            );
            assert_eq!(
                render_inline_latex(&repeated_scripts).chars().next(),
                Some('x')
            );
        })
        .expect("spawn bounded-stack renderer")
        .join()
        .expect("deeply nested rendering must not panic");
}

#[test]
fn renders_braced_matrices_and_cases_with_their_distinct_delimiters() {
    let matrix = render_display_latex(r"\begin{Bmatrix}a & b \\ c & d\end{Bmatrix}");
    assert!(
        matrix
            .first()
            .is_some_and(|line| line.starts_with('⎧') && line.ends_with('⎫'))
    );
    assert!(
        matrix
            .last()
            .is_some_and(|line| line.starts_with('⎩') && line.ends_with('⎭'))
    );

    let cases = render_display_latex(r"\begin{cases}x & x>0 \\ -x & x<0\end{cases}");
    assert!(cases.iter().all(|line| !line.ends_with(['⎫', '⎬', '⎭'])));
}

#[test]
fn matrix_scanner_respects_nested_environments_escapes_and_spacing() {
    assert_eq!(
        render_inline_latex(
            r"\begin{bmatrix}\begin{bmatrix}a & b \\ c & d\end{bmatrix}\end{bmatrix}"
        ),
        "[[a, b; c, d]]"
    );
    assert_eq!(
        render_inline_latex(r"\begin{bmatrix}a \& b & c\end{bmatrix}"),
        "[a & b, c]"
    );
    assert_eq!(
        render_inline_latex(r"\begin{array}{cc}a & b\end{array}"),
        "a, b"
    );
    assert_eq!(
        render_inline_latex(r"\begin{bmatrix}a \\[4pt] b \\\end{bmatrix}"),
        "[a; b]"
    );
}

#[test]
fn unmatched_environment_is_preserved_without_inventing_an_end_marker() {
    let source = r"\begin{bmatrix}a & b";
    assert_eq!(render_inline_latex(source), source);
}

#[test]
fn latexish_fuzz_corpus_never_panics_or_loses_all_visible_content() {
    const ALPHABET: &[char] = &[
        '\\', '{', '}', '[', ']', '^', '_', '&', '$', ' ', '\n', 'x', '7', 'α', '界',
    ];
    let mut state = 0x9e37_79b9_u32;
    for case in 0..256 {
        let mut source = String::new();
        for _ in 0..(32 + case % 96) {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            source.push(ALPHABET[state as usize % ALPHABET.len()]);
        }
        let inline = render_inline_latex(&source);
        let display = render_display_latex(&source);
        if !source.trim().is_empty() {
            assert!(!inline.trim().is_empty(), "case {case}: {source:?}");
            assert!(
                display.iter().any(|line| !line.trim().is_empty()),
                "case {case}: {source:?}"
            );
        }
    }
}
