#[test]
fn test_reasoning_line_survives_embedded_markdown() {
    // Adversarial corpus: each of these contains markdown the renderer would
    // normally act on (emphasis, code, links, headings, lists, math, html).
    // After wrapping with reasoning_line_markup they must all render as a single
    // dim+italic run with no characters lost.
    let cases = [
        "plain reasoning with no markup",
        "it's a contraction with an apostrophe",
        "emphasis *star* and _under_ score",
        "double **bold** and __strong__ markers",
        "inline `code span` should stay literal",
        "a [link](https://example.com) reference",
        "an ![image](pic.png) embed",
        "trailing single asterisk like 5 * 3 = 15",
        "ratio a/b and pipe a | b in a table-ish line",
        "tilde ~~strike~~ and lone ~ tilde",
        "angle <tag> and ampersand a & b",
        "leading # not a heading and - not a list",
        "math $x^2$ and $$y$$ dollar spans",
        "backslash path C:\\\\Users\\\\name",
        "unbalanced *open emphasis that never closes",
        "unbalanced `open code that never closes",
        "mixed it's **bold** with `code` and a [link](x)",
        "🤔 unicode and emoji with it's apostrophe",
        // Whitespace edges: CommonMark emphasis flanking rules reject a closing
        // `*` preceded by whitespace (or an opening `*` followed by whitespace),
        // so lines that start/end with whitespace must still stay dim+italic.
        "trailing spaces break the closing star   ",
        "   leading spaces before the text",
        "  surrounded by spaces on both sides  ",
        "trailing tab breaks it\t",
        "ends right at an asterisk *",
        "* starts at an asterisk",
        "   ",
        "\t",
        "ends with escaped backslash and space \\ ",
    ];
    for case in cases {
        assert_reasoning_line_fully_dim_italic(case);
    }
}

#[test]
fn test_multiline_reasoning_renders_one_visual_line_per_thought() {
    // Regression: consecutive reasoning lines must NOT collapse into one run-on
    // paragraph. Each `reasoning_line_markup` line ends in a CommonMark hard
    // break so the model's line structure is preserved (one visual row per
    // thought) instead of softbreaks rendering as spaces.
    let thoughts = [
        "First I will analyze the problem.",
        "Then I will consider the options.",
        "Finally I will decide.",
    ];
    let mut md = String::new();
    for thought in thoughts {
        md.push_str(&crate::reasoning_line_markup(thought));
    }
    let lines = render_markdown(&md);

    // One rendered line per thought (ignoring any trailing blank lines).
    let non_blank: Vec<String> = lines
        .iter()
        .map(line_to_string)
        .filter(|t| !t.trim().is_empty())
        .collect();
    assert_eq!(
        non_blank.len(),
        thoughts.len(),
        "each reasoning thought should be its own line, got: {non_blank:?}"
    );
    for (rendered, expected) in non_blank.iter().zip(thoughts) {
        assert_eq!(
            rendered.trim(),
            expected,
            "reasoning line content mismatch: {non_blank:?}"
        );
    }

    // And every visible span must still be dim + italic.
    let dim = md_dim_color();
    for line in &lines {
        for span in &line.spans {
            if span.content.trim().is_empty() {
                continue;
            }
            assert_eq!(
                span.style.fg,
                Some(dim),
                "reasoning span not dim: {:?}",
                span.content
            );
            assert!(
                span.style.add_modifier.contains(Modifier::ITALIC),
                "reasoning span not italic: {:?}",
                span.content
            );
        }
    }
}

#[test]
fn test_reasoning_emphasis_does_not_leak_into_following_text() {
    // After the reasoning emphasis closes, normal paragraph text must not be
    // styled as reasoning (dim/italic).
    let md = format!(
        "{}\n{}",
        crate::reasoning_line_markup("it's **thinking** with `code`."),
        "Normal answer text."
    );
    let lines = render_markdown(&md);

    let dim = md_dim_color();
    let answer_line = lines
        .iter()
        .find(|l| line_to_string(l).contains("Normal answer text."))
        .expect("answer line present");
    for span in &answer_line.spans {
        if span.content.trim().is_empty() {
            continue;
        }
        assert_ne!(
            span.style.fg,
            Some(dim),
            "answer span must not be dim: {:?}",
            span.content
        );
        assert!(
            !span.style.add_modifier.contains(Modifier::ITALIC),
            "answer span must not be italic: {:?}",
            span.content
        );
    }
}

#[test]
fn test_reasoning_summary_line_markup_folds_to_single_dim_italic_trace() {
    let sentinel = crate::REASONING_SENTINEL;

    // Pluralized count for multi-line blocks.
    let many = crate::reasoning_summary_line_markup(3);
    assert!(
        many.contains(&format!("*{0}▸ thought (3 lines){0}*", sentinel)),
        "expected pluralized summary markup, got: {many:?}"
    );

    // Single/zero-line blocks omit the count.
    let one = crate::reasoning_summary_line_markup(1);
    assert!(
        one.contains(&format!("*{0}▸ thought{0}*", sentinel)) && !one.contains("lines"),
        "expected bare summary markup, got: {one:?}"
    );
    let none = crate::reasoning_summary_line_markup(0);
    assert!(
        none.contains(&format!("*{0}▸ thought{0}*", sentinel)),
        "{none:?}"
    );

    // The summary line renders dim + italic with no sentinel leaking into text.
    let lines = render_markdown(&many);
    let dim = md_dim_color();
    let mut saw_marker = false;
    for rendered in &lines {
        for span in &rendered.spans {
            assert!(
                !span.content.contains(sentinel),
                "sentinel leaked into visible summary: {:?}",
                span.content
            );
            if span.content.trim().is_empty() {
                continue;
            }
            if span.content.contains('▸') {
                saw_marker = true;
            }
            assert_eq!(
                span.style.fg,
                Some(dim),
                "summary span not dim: {:?}",
                span.content
            );
            assert!(
                span.style.add_modifier.contains(Modifier::ITALIC),
                "summary span not italic: {:?}",
                span.content
            );
        }
    }
    assert!(saw_marker, "summary marker '▸' must be visible: {lines:?}");
}
