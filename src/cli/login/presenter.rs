//! Terminal presentation for shared authentication flows.

use super::*;

/// Terminal implementation of the shared auth flows' login presenter.
struct TerminalLoginPresenter;

impl crate::auth::login_ui::LoginPresenter for TerminalLoginPresenter {
    fn qr_section(
        &self,
        data: &str,
        heading: &str,
        indent: &str,
        browser_suppressed: bool,
    ) -> Option<String> {
        jcode_terminal_prompt::qr::indented_section(data, heading, indent, browser_suppressed)
    }

    fn read_secret_line(&self) -> Result<String> {
        jcode_terminal_prompt::read_secret_line()
    }
}

pub fn install_terminal_login_presenter() {
    crate::auth::login_ui::install(Box::new(TerminalLoginPresenter));
}

pub(super) fn read_line_trimmed(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub(super) fn maybe_open_browser(target: &str, no_browser: bool) -> bool {
    if crate::auth::browser_suppressed(no_browser) {
        false
    } else {
        open::that(target).is_ok()
    }
}

/// Hide URL userinfo, query and fragment in login output, not stored settings.
pub(super) fn display_url(raw: &str) -> String {
    let invalid = "[invalid URL]";
    let Ok(mut parsed) = reqwest::Url::parse(raw.trim()) else {
        return invalid.to_string();
    };
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || parsed.set_username("").is_err()
        || parsed.set_password(None).is_err()
    {
        return invalid.to_string();
    }
    parsed.set_query(None);
    parsed.set_fragment(None);
    parsed.to_string()
}

#[cfg(test)]
mod tests {
    use super::{display_url, maybe_open_browser};

    #[test]
    fn suppressed_login_browser_never_opens() {
        assert!(!maybe_open_browser("https://example.test/login", true));
    }

    #[test]
    fn login_url_display_removes_credential_components() {
        for (raw, expected) in [
            ("https://example.test/v1", "https://example.test/v1"),
            (
                "https://user:fixture-password@example.test/v1",
                "https://example.test/v1",
            ),
            (
                "https://example.test/v1?key=fixture-query#fixture-fragment",
                "https://example.test/v1",
            ),
            ("http://[::1]:8080/v1", "http://[::1]:8080/v1"),
            ("not a URL with fixture-password", "[invalid URL]"),
            ("file:///fixture-password", "[invalid URL]"),
            ("", "[invalid URL]"),
        ] {
            let original = raw.to_string();
            assert_eq!(display_url(&original), expected);
            assert_eq!(original, raw);
        }
    }
}
