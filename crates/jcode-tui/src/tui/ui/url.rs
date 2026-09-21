use regex::Regex;
use std::sync::OnceLock;
use unicode_width::UnicodeWidthStr;

pub(crate) fn url_regex() -> Option<&'static Regex> {
    static URL_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    URL_REGEX
        .get_or_init(|| Regex::new(r#"(?i)(?:https?://|mailto:|file://)[^\s<>'\"]+"#).ok())
        .as_ref()
}

fn markdown_link_regex() -> Option<&'static Regex> {
    static MARKDOWN_LINK_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    MARKDOWN_LINK_REGEX
        .get_or_init(|| Regex::new(r#"\[([^]\n]+)\]\(([^\s)]+)(?:\s+[^)]*)?\)"#).ok())
        .as_ref()
}

/// Absolute filesystem path. At least two segments are required so a slash
/// command such as `/transcript` is never treated as a link, and an optional
/// `:line[:column]` suffix stays inside the hit range while being stripped from
/// the returned target.
fn absolute_path_regex() -> Option<&'static Regex> {
    static ABSOLUTE_PATH_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    ABSOLUTE_PATH_REGEX
        .get_or_init(|| {
            Regex::new(
                r#"(?:^|[\s(\[`"'])(/[A-Za-z0-9_.\-+@%]+(?:/[A-Za-z0-9_.\-+@%]+)+)(?::[0-9]+(?::[0-9]+)?)?"#,
            )
            .ok()
        })
        .as_ref()
}

/// Repository-relative path. A separator and a file extension are both
/// required, so ordinary prose such as `and/or` or `read/write` cannot match.
fn relative_path_regex() -> Option<&'static Regex> {
    static RELATIVE_PATH_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    RELATIVE_PATH_REGEX
        .get_or_init(|| {
            Regex::new(
                r#"(?:^|[\s(\[`"'])((?:\.{1,2}/)?[A-Za-z0-9_\-]+(?:/[A-Za-z0-9_.\-]+)+\.[A-Za-z0-9]{1,10})(?::[0-9]+(?::[0-9]+)?)?"#,
            )
            .ok()
        })
        .as_ref()
}

/// Strip a trailing `:line[:column]` from a link target.
///
/// Deliberately leaves anything carrying a URL scheme alone, so an authority
/// such as `https://host:8080/x` keeps its port. An opener expects a filesystem
/// path, not a location suffix.
pub(crate) fn strip_location_suffix(target: &str) -> String {
    if target.contains("://") {
        return target.to_string();
    }
    let mut candidate = target;
    for _ in 0..2 {
        let Some(index) = candidate.rfind(':') else {
            break;
        };
        let (head, tail) = candidate.split_at(index);
        let digits = &tail[1..];
        if head.is_empty() || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            break;
        }
        candidate = head;
    }
    candidate.to_string()
}

pub(crate) fn trim_url_candidate(candidate: &str) -> &str {
    let mut trimmed = candidate;
    loop {
        let next = if trimmed.ends_with(['.', ',', ';', ':', '!', '?'])
            || (trimmed.ends_with(')')
                && trimmed.matches(')').count() > trimmed.matches('(').count())
            || (trimmed.ends_with(']')
                && trimmed.matches(']').count() > trimmed.matches('[').count())
            || (trimmed.ends_with('}')
                && trimmed.matches('}').count() > trimmed.matches('{').count())
        {
            &trimmed[..trimmed.len() - 1]
        } else {
            trimmed
        };

        if next.len() == trimmed.len() {
            return trimmed;
        }
        trimmed = next;
    }
}

pub(crate) fn link_target_for_display_column(raw_text: &str, column: usize) -> Option<String> {
    if let Some(regex) = markdown_link_regex() {
        for captures in regex.captures_iter(raw_text) {
            let (Some(whole), Some(target)) = (captures.get(0), captures.get(2)) else {
                continue;
            };
            // The whole `[label](target)` construct is a click target, not only
            // the label. The renderer appends a link's destination as its own
            // dim span, so the path the reader sees and naturally clicks is the
            // destination; restricting the hit range to the label left that
            // visible path inert.
            let start_col = raw_text[..whole.start()].width();
            let end_col = start_col + whole.as_str().width();
            if column >= start_col && column < end_col {
                return Some(target.as_str().to_string());
            }
        }
    }

    for mat in url_regex()?.find_iter(raw_text) {
        let matched = &raw_text[mat.start()..mat.end()];
        let trimmed = trim_url_candidate(matched);
        if trimmed.is_empty() {
            continue;
        }

        let start_col = raw_text[..mat.start()].width();
        let end_col = start_col + trimmed.width();
        if column >= start_col && column < end_col && ::url::Url::parse(trimmed).is_ok() {
            return Some(trimmed.to_string());
        }
    }

    // Bare paths last, so a Markdown link or a scheme URL always wins.
    for regex in [absolute_path_regex(), relative_path_regex()] {
        let Some(regex) = regex else {
            continue;
        };
        for captures in regex.captures_iter(raw_text) {
            let (Some(whole), Some(path)) = (captures.get(0), captures.get(1)) else {
                continue;
            };
            // Start at the path itself, not at the boundary character the regex
            // consumed to anchor the match, so the preceding space is not a hit.
            let start_col = raw_text[..path.start()].width();
            let end_col = raw_text[..whole.end()].width();
            if column >= start_col && column < end_col {
                return Some(strip_location_suffix(path.as_str()));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        link_target_for_display_column, strip_location_suffix, trim_url_candidate, url_regex,
    };

    #[test]
    fn url_regex_matches_supported_link_schemes() {
        let regex = url_regex();
        assert!(regex.is_some(), "test URL regex should initialize");
        let Some(regex) = regex else {
            return;
        };
        let text = "See https://example.com, mailto:user@example.com, and file:///tmp/a.txt";
        let matches: Vec<&str> = regex.find_iter(text).map(|mat| mat.as_str()).collect();

        assert_eq!(
            matches,
            vec![
                "https://example.com,",
                "mailto:user@example.com,",
                "file:///tmp/a.txt"
            ]
        );
    }

    #[test]
    fn trim_url_candidate_removes_trailing_sentence_punctuation() {
        assert_eq!(
            trim_url_candidate("https://example.com,"),
            "https://example.com"
        );
        assert_eq!(
            trim_url_candidate("https://example.com?!"),
            "https://example.com"
        );
        assert_eq!(
            trim_url_candidate("mailto:user@example.com."),
            "mailto:user@example.com"
        );
    }

    #[test]
    fn trim_url_candidate_preserves_balanced_closing_delimiters() {
        assert_eq!(
            trim_url_candidate("https://example.com/path_(draft)"),
            "https://example.com/path_(draft)"
        );
        assert_eq!(
            trim_url_candidate("https://example.com/path_(draft))."),
            "https://example.com/path_(draft)"
        );
        assert_eq!(
            trim_url_candidate("https://example.com/[docs]]"),
            "https://example.com/[docs]"
        );
    }

    #[test]
    fn link_target_for_display_column_returns_trimmed_url_when_inside_url() {
        let text = "Open https://example.com/docs, please";

        assert_eq!(
            link_target_for_display_column(text, "Open https://example".len()),
            Some("https://example.com/docs".to_string())
        );
        assert_eq!(
            link_target_for_display_column(text, "Open ".len() - 1),
            None
        );
        assert_eq!(
            link_target_for_display_column(text, "Open https://example.com/docs".len()),
            None
        );
    }

    #[test]
    fn link_target_for_display_column_uses_display_width_for_wide_prefixes() {
        let text = "🙂 https://example.com";

        assert_eq!(
            link_target_for_display_column(text, 3),
            Some("https://example.com".to_string())
        );
        assert_eq!(link_target_for_display_column(text, 1), None);
    }

    #[test]
    fn link_target_for_display_column_resolves_markdown_label() {
        let text = "Read the [guide](docs/guide.md#setup) today";

        assert_eq!(
            link_target_for_display_column(text, 10),
            Some("docs/guide.md#setup".to_string())
        );
        assert_eq!(link_target_for_display_column(text, 8), None);
    }

    #[test]
    fn markdown_link_destination_is_also_a_click_target() {
        // The renderer appends a link's destination as its own dim span, so the
        // reader naturally clicks the destination rather than the label. Column
        // 20 is inside `docs/guide.md`, past the label.
        let text = "Read the [guide](docs/guide.md#setup) today";

        assert_eq!(
            link_target_for_display_column(text, 20),
            Some("docs/guide.md#setup".to_string())
        );
        assert_eq!(link_target_for_display_column(text, 37), None);
    }

    #[test]
    fn bare_absolute_path_is_a_click_target() {
        let text = "see /Users/x/y.rs now";

        assert_eq!(
            link_target_for_display_column(text, 13),
            Some("/Users/x/y.rs".to_string())
        );
        assert_eq!(link_target_for_display_column(text, 2), None);
    }

    #[test]
    fn bare_relative_path_is_a_click_target() {
        let text = "edited docs/plan.md today";

        assert_eq!(
            link_target_for_display_column(text, 12),
            Some("docs/plan.md".to_string())
        );
        // The boundary space the regex consumed is not part of the hit range.
        assert_eq!(link_target_for_display_column(text, 6), None);
    }

    #[test]
    fn path_location_suffix_is_clickable_but_stripped_from_the_target() {
        let text = "at crates/x/y.rs:12:5 now";

        assert_eq!(
            link_target_for_display_column(text, 19),
            Some("crates/x/y.rs".to_string())
        );
        assert_eq!(
            link_target_for_display_column(text, 12),
            Some("crates/x/y.rs".to_string())
        );
    }

    #[test]
    fn prose_with_a_slash_is_not_a_link() {
        let text = "either and/or read/write maybe";

        assert_eq!(link_target_for_display_column(text, 10), None);
    }

    #[test]
    fn slash_command_is_not_a_link() {
        assert_eq!(link_target_for_display_column("/transcript", 3), None);
    }

    #[test]
    fn url_with_a_port_is_returned_unchanged() {
        let text = "see https://host:8080/x now";

        assert_eq!(
            link_target_for_display_column(text, 22),
            Some("https://host:8080/x".to_string())
        );
    }

    #[test]
    fn strip_location_suffix_leaves_a_url_port_alone() {
        assert_eq!(strip_location_suffix("crates/x/y.rs:12:5"), "crates/x/y.rs");
        assert_eq!(strip_location_suffix("crates/x/y.rs"), "crates/x/y.rs");
        assert_eq!(
            strip_location_suffix("https://host:8080/x"),
            "https://host:8080/x"
        );
    }
}
