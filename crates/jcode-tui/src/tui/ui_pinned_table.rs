use crate::tui::{markdown, mermaid};
use ratatui::text::Line;

pub(crate) fn is_rendered_table_line(line: &Line<'_>) -> bool {
    let text: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    text.contains(" │ ") || text.contains("─┼─")
}

pub(super) fn wrap_side_panel_markdown_lines(
    lines: Vec<Line<'static>>,
    width: usize,
) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .flat_map(|line| {
            if is_rendered_table_line(&line)
                || mermaid::parse_image_placeholder(&line).is_some()
                || mermaid::parse_inline_image_placeholder(&line).is_some()
            {
                vec![line]
            } else {
                markdown::wrap_line(line, width)
            }
        })
        .collect()
}
