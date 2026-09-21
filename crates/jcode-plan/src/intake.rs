//! Deterministic work-packet intake classifier.
//!
//! Pure string rules over packet text plus a caller-supplied
//! [`DataClass`]. No model calls, no network, no protocol surfaces.

use jcode_attempt_types::DataClass;
use serde::Deserialize;
use serde::Serialize;

/// Closed set of intake task shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskClass {
    Mechanical,
    Implement,
    Fix,
    Explore,
    Review,
    Research,
    Docs,
    Infra,
    Prompt,
}

impl TaskClass {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskClass::Mechanical => "mechanical",
            TaskClass::Implement => "implement",
            TaskClass::Fix => "fix",
            TaskClass::Explore => "explore",
            TaskClass::Review => "review",
            TaskClass::Research => "research",
            TaskClass::Docs => "docs",
            TaskClass::Infra => "infra",
            TaskClass::Prompt => "prompt",
        }
    }
}

/// How much judgment the work requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Judgment {
    None,
    Bounded,
    Frontier,
}

impl Judgment {
    pub fn as_str(self) -> &'static str {
        match self {
            Judgment::None => "none",
            Judgment::Bounded => "bounded",
            Judgment::Frontier => "frontier",
        }
    }
}

/// Where the work may execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Containment {
    None,
    Worktree,
    Sandbox,
}

impl Containment {
    pub fn as_str(self) -> &'static str {
        match self {
            Containment::None => "none",
            Containment::Worktree => "worktree",
            Containment::Sandbox => "sandbox",
        }
    }
}

/// Coarse size hint from the packet text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffSize {
    Tiny,
    Small,
    Large,
}

/// Features extracted from packet text by string rules only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntakeFeatures {
    pub paths: Vec<String>,
    pub has_test_command: bool,
    pub touches_secret_path: bool,
    pub diff_size_hint: DiffSize,
    pub mentions: Vec<String>,
}

const SECRET_PATH_MARKERS: &[&str] = &[
    "auth.json",
    ".env",
    "keychain",
    "credentials",
    "id_rsa",
    ".pem",
];

const TEST_COMMANDS: &[&str] = &["cargo test", "npm test", "pytest"];

const PATHLIKE_CHARS: &[char] = &['/', '.', '-', '_'];

const MECHANICAL_KEYWORDS: &[&str] = &[
    "rename",
    "reformat",
    "rustfmt",
    "typo",
    "mechanical",
    "sweep",
];
const FIX_KEYWORDS: &[&str] = &[
    "fix",
    "bug",
    "regression",
    "broken",
    "crash",
    "panic",
    "fails",
];
const EXPLORE_KEYWORDS: &[&str] = &["explore", "investigate", "look into", "survey", "map out"];
const REVIEW_KEYWORDS: &[&str] = &["review", "critique", "audit", "assess"];
const RESEARCH_KEYWORDS: &[&str] = &[
    "research",
    "compare",
    "evaluate options",
    "tradeoff",
    "study",
];
const DOCS_KEYWORDS: &[&str] = &[
    "docs",
    "documentation",
    "readme",
    "changelog",
    "comment",
    "adoc",
];
const INFRA_KEYWORDS: &[&str] = &[
    "ci",
    "build",
    "infra",
    "pipeline",
    "workflows",
    "docker",
    "deps",
    "upgrade",
    "toolchain",
];
const PROMPT_KEYWORDS: &[&str] = &["prompt", "skill", "instructions", "agent packet"];

fn count_keyword_hits(text: &str, keywords: &[&str]) -> usize {
    keywords.iter().filter(|k| text.contains(**k)).count()
}

fn looks_pathlike(token: &str) -> bool {
    !token.is_empty()
        && token.len() > 3
        && token
            .chars()
            .all(|c| c.is_alphanumeric() || PATHLIKE_CHARS.contains(&c))
        && token.chars().any(|c| PATHLIKE_CHARS.contains(&c))
        && !token.contains("..")
}

/// Extract string-rule features from a packet text blob.
pub fn extract_features(packet_text: &str) -> IntakeFeatures {
    let text = packet_text.to_ascii_lowercase();
    let paths: Vec<String> = text
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')' | '"' | '`' | ';'))
        .filter(|t| looks_pathlike(t))
        .map(|t| t.trim_matches('.').to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let touches_secret_path = SECRET_PATH_MARKERS.iter().any(|m| text.contains(m))
        || paths
            .iter()
            .any(|p| SECRET_PATH_MARKERS.iter().any(|m| p.contains(m)));

    let has_test_command = TEST_COMMANDS.iter().any(|t| text.contains(t));

    let diff_size_hint = if packet_text.len() < 400 {
        DiffSize::Tiny
    } else if packet_text.len() < 3000 {
        DiffSize::Small
    } else {
        DiffSize::Large
    };

    let mut mentions = Vec::new();
    for (list, tag) in [
        (MECHANICAL_KEYWORDS, "mechanical"),
        (FIX_KEYWORDS, "fix"),
        (EXPLORE_KEYWORDS, "explore"),
        (REVIEW_KEYWORDS, "review"),
        (RESEARCH_KEYWORDS, "research"),
        (DOCS_KEYWORDS, "docs"),
        (INFRA_KEYWORDS, "infra"),
        (PROMPT_KEYWORDS, "prompt"),
    ] {
        if count_keyword_hits(&text, list) > 0 {
            mentions.push(tag.to_string());
        }
    }

    IntakeFeatures {
        paths,
        has_test_command,
        touches_secret_path,
        diff_size_hint,
        mentions,
    }
}

/// Deterministic classification from extracted features.
pub fn classify(features: &IntakeFeatures, data_class: DataClass) -> IntakeDecision {
    let mut reasons = Vec::new();

    if data_class == DataClass::Secret || features.touches_secret_path {
        reasons.push("secret path or data class: sandbox + frontier".to_string());
        return IntakeDecision {
            task_class: TaskClass::Fix,
            judgment: Judgment::Frontier,
            containment: Containment::Sandbox,
            reasons,
        };
    }

    let mentions_any = |tags: &[&str]| {
        tags.iter()
            .any(|t| features.mentions.iter().any(|m| m == t))
    };

    let task_class = if mentions_any(&["prompt"]) {
        TaskClass::Prompt
    } else if mentions_any(&["docs"]) {
        TaskClass::Docs
    } else if mentions_any(&["mechanical"]) {
        TaskClass::Mechanical
    } else if mentions_any(&["review"]) {
        TaskClass::Review
    } else if mentions_any(&["research"]) {
        TaskClass::Research
    } else if mentions_any(&["explore"]) {
        TaskClass::Explore
    } else if mentions_any(&["infra"]) {
        TaskClass::Infra
    } else if mentions_any(&["fix"]) {
        TaskClass::Fix
    } else {
        TaskClass::Implement
    };

    // Infra/Fix-style work runs isolated in a worktree when it has a test command.
    let containment =
        if matches!(task_class, TaskClass::Infra | TaskClass::Fix) && features.has_test_command {
            reasons.push("infra/fix with test command: worktree".to_string());
            Containment::Worktree
        } else if matches!(
            task_class,
            TaskClass::Docs | TaskClass::Prompt | TaskClass::Explore | TaskClass::Research
        ) {
            reasons.push("read-mostly or text surface: no special containment".to_string());
            Containment::None
        } else {
            Containment::None
        };

    let judgment = match task_class {
        TaskClass::Mechanical => {
            reasons.push("mechanical: judgment none".to_string());
            Judgment::None
        }
        TaskClass::Review | TaskClass::Research => {
            reasons.push("review/research: judgment bounded".to_string());
            Judgment::Bounded
        }
        TaskClass::Implement | TaskClass::Infra | TaskClass::Fix => {
            if features.diff_size_hint == DiffSize::Tiny {
                reasons.push("implement/fix/infra with tiny diff: judgment none".to_string());
                Judgment::None
            } else {
                reasons.push("implement/infra non-tiny: judgment frontier".to_string());
                Judgment::Frontier
            }
        }
        _ => {
            reasons.push("exploratory or text work: judgment bounded".to_string());
            Judgment::Bounded
        }
    };

    IntakeDecision {
        task_class,
        judgment,
        containment,
        reasons,
    }
}

/// The classifier's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntakeDecision {
    pub task_class: TaskClass,
    pub judgment: Judgment,
    pub containment: Containment,
    pub reasons: Vec<String>,
}
