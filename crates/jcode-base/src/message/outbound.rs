//! Shared, fail-closed text boundary. This is not a general-purpose PII detector.
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

const SECRET: &str = "[REDACTED_SECRET]";
const CARD: &str = "[REDACTED_CARD]";
const MAX_DEPTH: usize = 64;

/// Deliberately contains no input, regex source, field name, or underlying error.
#[derive(Clone, Copy, Debug)]
pub struct SanitizeError;

impl std::fmt::Display for SanitizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("outbound sanitizer failed; content withheld")
    }
}
impl std::error::Error for SanitizeError {}

fn compile(pattern: &str) -> Result<Regex, SanitizeError> {
    Regex::new(pattern).map_err(|_| SanitizeError)
}

struct Rules {
    direct: Vec<Regex>,
    assignments: Regex,
    headers: Regex,
    secret_key: Regex,
    cards: Regex,
}

impl Rules {
    fn new() -> Result<Self, SanitizeError> {
        Ok(Self {
            direct: [
                r"sk-(?:ant-(?:oat|ort)01-|or-v1-)?[A-Za-z0-9_-]{20,}",
                r"gh[pousr]_[A-Za-z0-9]{20,}",
                r"github_pat_[A-Za-z0-9_]{20,}",
                r"ya29\.[A-Za-z0-9._-]{20,}",
                r"AIza[0-9A-Za-z_-]{20,}",
                r"xox[baprs]-[A-Za-z0-9-]{10,}",
                r"(?:AKIA|ASIA)[0-9A-Z]{16}",
                r"(?i)Bearer[ \t]+[A-Za-z0-9._~+/=-]{20,}",
                r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
                r"(?s)-----BEGIN (?:(?:RSA|EC|OPENSSH|DSA|PGP|ENCRYPTED) )?PRIVATE KEY-----.*?(?:-----END (?:(?:RSA|EC|OPENSSH|DSA|PGP|ENCRYPTED) )?PRIVATE KEY-----|\z)",
            ].iter().map(|p| compile(p)).collect::<Result<_, _>>()?,
            // Match each assignment, not the rest of the line. JSON is handled
            // structurally below. Plain assignments retain the legacy marker.
            assignments: assignment_pattern(r"[A-Za-z0-9_]*(?:API_KEY|TOKEN|SECRET|PASSWORD|COOKIE|AUTHORIZATION)")?,
            // Authentication schemes and cookie lists contain spaces and
            // semicolons. A single-word assignment match would leak the rest.
            headers: compile(r#"(?im)(\b(?:[A-Z0-9_]*AUTHORIZATION|[A-Z0-9_]*COOKIE|SET-COOKIE)["']?[ \t]*[:=][ \t]*)[^\r\n]+"#)?,
            secret_key: compile(r"(?i)(?:api[_-]?key|token|secret|password|cookie|authorization)$")?,
            // Only conventional card groupings, never arbitrary lists of numbers.
            cards: compile(r"\b(?:[0-9]{4}(?:[ -][0-9]{4}){3}[ -][0-9]{1,3}|[0-9]{4}(?:[ -][0-9]{4}){3}|[0-9]{4}[ -][0-9]{6}[ -][0-9]{4,5}|[0-9]{4}[ -][0-9]{4}[ -][0-9]{5}|[0-9]{13,19})\b")?,
        })
    }
}

fn assignment_pattern(key: &str) -> Result<Regex, SanitizeError> {
    compile(&format!(
        r#"(?i)(\b(?:{key})["']?[ \t]*[:=][ \t]*)(?:\[REDACTED_SECRET\]|"[^"\r\n]*"|'[^'\r\n]*'|[^\s,;\}}\]\"']+)"#
    ))
}

struct Sanitizer {
    rules: &'static Rules,
    custom: Vec<(String, Regex)>,
}

impl Sanitizer {
    fn new() -> Result<Self, SanitizeError> {
        static RULES: OnceLock<Result<Rules, SanitizeError>> = OnceLock::new();
        let rules = RULES
            .get_or_init(Rules::new)
            .as_ref()
            .map_err(|_| SanitizeError)?;
        let mut custom = Vec::new();
        for source in [
            "JCODE_OPENROUTER_API_KEY_NAME",
            "JCODE_OPENAI_COMPAT_API_KEY_NAME",
        ] {
            let name = match std::env::var(source) {
                Ok(name) => name.trim().to_string(),
                Err(std::env::VarError::NotPresent) => continue,
                Err(_) => return Err(SanitizeError),
            };
            if name.is_empty() {
                continue;
            }
            if !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
                return Err(SanitizeError);
            }
            let pattern = assignment_pattern(&regex::escape(&name))?;
            custom.push((name, pattern));
        }
        Ok(Self { rules, custom })
    }

    fn text(&self, text: &str, depth: usize) -> Result<String, SanitizeError> {
        if depth > MAX_DEPTH {
            return Err(SanitizeError);
        }
        // Tool arguments and provider-native extensions may themselves be JSON
        // strings. Decode before scanning, including escaped digits/credentials.
        let trimmed = text.trim_start();
        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && let Ok(mut value) = serde_json::from_str::<Value>(text)
        {
            let original = value.clone();
            self.json(&mut value, depth + 1)?;
            if value != original {
                return serde_json::to_string(&value).map_err(|_| SanitizeError);
            }
            return Ok(text.to_owned());
        }
        let mut clean = text.to_owned();
        for pattern in &self.rules.direct {
            clean = pattern.replace_all(&clean, SECRET).into_owned();
        }
        for pattern in std::iter::once(&self.rules.headers)
            .chain(std::iter::once(&self.rules.assignments))
            .chain(self.custom.iter().map(|(_, re)| re))
        {
            clean = pattern
                .replace_all(&clean, "${1}[REDACTED_SECRET]")
                .into_owned();
        }
        Ok(self
            .rules
            .cards
            .replace_all(&clean, |caps: &regex::Captures<'_>| {
                let m = caps.get(0).expect("whole match");
                let adjacent_identifier = clean[..m.start()]
                    .chars()
                    .next_back()
                    .into_iter()
                    .chain(clean[m.end()..].chars().next())
                    .any(|c| c.is_alphanumeric() || c == '_');
                if !adjacent_identifier && is_card(m.as_str()) {
                    CARD.to_owned()
                } else {
                    m.as_str().to_owned()
                }
            })
            .into_owned())
    }

    fn json(&self, value: &mut Value, depth: usize) -> Result<(), SanitizeError> {
        if depth > MAX_DEPTH {
            return Err(SanitizeError);
        }
        match value {
            Value::String(text) => *text = self.text(text, depth + 1)?,
            Value::Number(number) if number.as_u64().is_some() && is_card(&number.to_string()) => {
                *value = Value::String(CARD.into());
            }
            Value::Array(items) => {
                for item in items {
                    self.json(item, depth + 1)?;
                }
            }
            Value::Object(object) => {
                let mut clean = serde_json::Map::new();
                for (key, mut item) in std::mem::take(object) {
                    if item.is_string()
                        && (self.rules.secret_key.is_match(&key)
                            || self
                                .custom
                                .iter()
                                .any(|(name, _)| name.eq_ignore_ascii_case(&key)))
                    {
                        item = Value::String(SECRET.into());
                    } else {
                        self.json(&mut item, depth + 1)?;
                    }
                    let key = self.text(&key, depth + 1)?;
                    if clean.insert(key, item).is_some() {
                        return Err(SanitizeError);
                    }
                }
                *object = clean;
            }
            _ => {}
        }
        Ok(())
    }
}

fn is_card(candidate: &str) -> bool {
    let digits: Vec<u8> = candidate
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|c| c - b'0')
        .collect();
    if !(13..=19).contains(&digits.len()) {
        return false;
    }
    let prefix = |n: usize| {
        digits
            .iter()
            .take(n)
            .fold(0u32, |v, d| v * 10 + u32::from(*d))
    };
    let issuer = match digits.len() {
        13 => digits[0] == 4,
        14 => (300..=305).contains(&prefix(3)) || matches!(prefix(2), 36 | 38 | 39),
        15 => matches!(prefix(2), 34 | 37),
        16..=19 => {
            (digits[0] == 4 && matches!(digits.len(), 16 | 19))
                || (digits.len() == 16
                    && ((51..=55).contains(&prefix(2)) || (2221..=2720).contains(&prefix(4))))
                || ((digits.len() == 16 || digits.len() == 19)
                    && (prefix(4) == 6011 || prefix(2) == 65 || (644..=649).contains(&prefix(3))))
                || (3528..=3589).contains(&prefix(4))
                || prefix(2) == 62
        }
        _ => false,
    };
    issuer
        && digits
            .iter()
            .rev()
            .enumerate()
            .map(|(i, d)| {
                let n = if i % 2 == 1 { d * 2 } else { *d };
                u32::from(if n > 9 { n - 9 } else { n })
            })
            .sum::<u32>()
            % 10
            == 0
}

pub fn sanitize_outbound_text(text: &str) -> Result<String, SanitizeError> {
    Sanitizer::new()?.text(text, 0)
}

/// Sanitizes every string, including keys and serialized JSON arguments. On
/// failure the caller must discard the value, which may be partially modified.
pub fn sanitize_outbound_json(value: &mut Value) -> Result<(), SanitizeError> {
    Sanitizer::new()?.json(value, 0)
}

#[cfg(test)]
mod tests;
