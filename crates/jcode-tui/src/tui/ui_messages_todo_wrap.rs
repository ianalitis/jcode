fn todo_card_line(
    spans: Vec<Span<'static>>,
    base_indent: &str,
    inner_width: usize,
) -> Line<'static> {
    let mut prefixed = vec![Span::raw(base_indent.to_string())];
    prefixed.extend(spans);
    super::truncate_line_with_ellipsis_to_width(
        &Line::from(prefixed),
        inner_width.saturating_add(base_indent.width()),
    )
}

/// Wrap one labeled detail line to the card width.
fn push_todo_wrapped_detail(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: &str,
    base_indent: &str,
    inner_width: usize,
) {
    let prefix = format!("  {} · ", label);
    let prefix_width = prefix.width();
    let available = inner_width.saturating_sub(prefix_width).max(1);
    for (index, chunk) in wrap_todo_detail(value, available).into_iter().enumerate() {
        lines.push(todo_card_line(
            vec![
                Span::styled(
                    if index == 0 {
                        prefix.clone()
                    } else {
                        " ".repeat(prefix_width)
                    },
                    Style::default().fg(todo_label_color()),
                ),
                Span::styled(chunk, Style::default().fg(todo_meta_color())),
            ],
            base_indent,
            inner_width,
        ));
    }
}
