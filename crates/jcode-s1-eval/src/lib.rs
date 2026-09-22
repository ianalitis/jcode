//! Deterministic baseline (arm A) and evaluation harness for the System 1
//! comparison on `factory.intake.classify`. See `docs/P2_S1_EVAL_PACKET.md`.
//!
//! This crate is pure: no network, no shell, no model calls. Model arms plug
//! in through [`Classifier`] and are scored by the same [`score`] function
//! against the same fixtures, so a lossy or lucky arm cannot hide behind a
//! different metric.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod decision;

pub use decision::{
    ADVISORY_AUTHORITY, DecisionArm, DecisionError, DecisionKind, DecisionOption, DecisionRequest,
    DecisionResult, DeterministicDecisionBaseline, MAX_OPTIONS, MIN_OPTIONS, OptionScore,
    UNCALIBRATED, validate as validate_decision, validate_request as validate_decision_request,
};

/// The anti-hallucination seam, shared by [`validate`] and
/// [`validate_decision`]: a returned span is only admissible when it is a
/// literal, non-empty substring of the input it claims to come from.
pub(crate) fn is_grounded(haystack: &str, span: &str) -> bool {
    !span.trim().is_empty() && haystack.contains(span)
}

// ---------------------------------------------------------------------------
// Closed candidate sets. Code owns these; no model may extend them.
// ---------------------------------------------------------------------------

pub const TASK_KINDS: &[&str] = &["brochure", "hybrid", "proposal"];
pub const COMPLEXITY: &[&str] = &["low", "medium", "high"];
pub const INDUSTRIES: &[&str] = &[
    "auto-repair",
    "restaurant",
    "religious-org",
    "insurance",
    "construction",
    "fisheries",
    "cleaning",
    "law",
    "medical",
    "other",
];
/// Skin ids are supplied by the caller from the factory's `starter/skins`.
/// The two shipped today are listed for fixtures; tests pass explicit sets.
pub const DEFAULT_SKINS: &[&str] = &["workshop-index", "editorial-record"];

/// Normalized intake input. Mirrors a subset of the factory `answers` shape
/// but is independent of it, so fixtures never contain private schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntakeCase {
    pub id: String,
    #[serde(default)]
    pub org_name: String,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub audience: String,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    /// Free text the operator or client appended. This is where injected
    /// instructions and negations live in the adversarial fixtures.
    #[serde(default)]
    pub notes: String,
}

impl IntakeCase {
    /// All free text joined, lowercased, for keyword rules.
    fn corpus(&self) -> String {
        let mut s = String::new();
        for part in [
            &self.org_name,
            &self.about,
            &self.audience,
            &self.goal,
            &self.notes,
        ] {
            s.push_str(part);
            s.push(' ');
        }
        for v in self.services.iter().chain(self.features.iter()) {
            s.push_str(v);
            s.push(' ');
        }
        s.to_lowercase()
    }
}

/// Output every arm must produce. Enums are validated against the closed
/// sets by [`validate`] before scoring; an invalid output scores as wrong.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Classification {
    pub task_kind: String,
    pub industry: String,
    pub complexity: String,
    #[serde(default)]
    pub skin_candidates: Vec<String>,
    #[serde(default)]
    pub abstain: bool,
    /// Field name to a quoted span from the input that supports the value.
    #[serde(default)]
    pub grounding: BTreeMap<String, String>,
}

/// Gold label for a case. `industry: None` means the correct answer is to
/// abstain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub task_kind: Option<String>,
    pub industry: Option<String>,
    pub complexity: Option<String>,
    #[serde(default)]
    pub acceptable_skins: Vec<String>,
    /// Ids that must never appear (for injection fixtures).
    #[serde(default)]
    pub forbidden_skins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    UnknownTaskKind(String),
    UnknownIndustry(String),
    UnknownComplexity(String),
    UnknownSkin(String),
    UngroundedField(String),
}

/// Deterministic validation: closed sets, and every grounding span must be a
/// literal substring of the input. Runs on every arm's output.
pub fn validate(
    case: &IntakeCase,
    out: &Classification,
    skins: &[&str],
) -> Result<(), ValidationError> {
    if out.abstain {
        return Ok(());
    }
    if !TASK_KINDS.contains(&out.task_kind.as_str()) {
        return Err(ValidationError::UnknownTaskKind(out.task_kind.clone()));
    }
    if !INDUSTRIES.contains(&out.industry.as_str()) {
        return Err(ValidationError::UnknownIndustry(out.industry.clone()));
    }
    if !COMPLEXITY.contains(&out.complexity.as_str()) {
        return Err(ValidationError::UnknownComplexity(out.complexity.clone()));
    }
    for s in &out.skin_candidates {
        if !skins.contains(&s.as_str()) {
            return Err(ValidationError::UnknownSkin(s.clone()));
        }
    }
    let haystack = serde_json::to_string(case).unwrap_or_default();
    for (field, span) in &out.grounding {
        if !is_grounded(&haystack, span) {
            return Err(ValidationError::UngroundedField(field.clone()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Arm A: deterministic rules
// ---------------------------------------------------------------------------

/// Any arm, including the deterministic baseline.
pub trait Classifier {
    fn name(&self) -> &str;
    fn classify(&self, case: &IntakeCase, skins: &[&str]) -> Classification;
}

/// Keyword and rule baseline. Negation-aware at the phrase level ("not a
/// restaurant" does not vote for restaurant). Abstains when nothing votes.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicBaseline;

const INDUSTRY_KEYWORDS: &[(&str, &[&str])] = &[
    (
        "auto-repair",
        &[
            "diesel",
            "truck",
            "engine",
            "mechanic",
            "auto repair",
            "repair shop",
            "transmission",
            "fleet",
            "brake",
            "tire",
            "garage",
            "vehicle",
            "automotive",
            "exhaust",
            "oil change",
            "taller mec",
            "reparación de auto",
        ],
    ),
    (
        "restaurant",
        &[
            "restaurant",
            "menu",
            "diner",
            "cafe",
            "bistro",
            "catering",
            "bakery",
            "kitchen",
        ],
    ),
    (
        "religious-org",
        &[
            "parish",
            "church",
            "orthodox",
            "liturgy",
            "feast",
            "bulletin",
            "congregation",
            "temple",
            "mosque",
            "worship",
            "prayer",
            "religious",
            "communauté religieuse",
            "culte",
            "iglesia",
            "ενορία",
            "εκκλησ",
        ],
    ),
    (
        "insurance",
        &[
            "insurance",
            "policy",
            "coverage",
            "claims",
            "premium",
            "underwrit",
            "seguro",
        ],
    ),
    (
        "construction",
        &[
            "paver",
            "brick",
            "patio",
            "masonry",
            "contractor",
            "remodel",
            "roofing",
            "retaining wall",
            "landscap",
            "construction",
            "construcción",
            "obras",
            "terraza",
            "amplía",
            "builder",
        ],
    ),
    (
        "fisheries",
        &[
            "fisher",
            "seafood",
            "fish ",
            "fishing",
            "wholesale catch",
            "dock",
            "pescad",
        ],
    ),
    (
        "cleaning",
        &["cleaning", "janitorial", "maid", "housekeeping", "limpieza"],
    ),
    (
        "law",
        &[
            "law firm",
            "attorney",
            "lawyer",
            "legal",
            "litigation",
            "counsel",
            "abogad",
            "avocat",
        ],
    ),
    (
        "medical",
        &[
            "clinic",
            "dental",
            "dentist",
            "physician",
            "medical",
            "chiropract",
            "therapy",
            "patient",
            "clínica",
            "médic",
        ],
    ),
];

const NEGATORS: &[&str] = &[
    "not a ",
    "not an ",
    "not ",
    "no ",
    "isn't a ",
    "is not a ",
    "never ",
];

/// Count keyword hits for one industry, skipping hits preceded by a negator
/// within a short window.
fn votes(corpus: &str, keywords: &[&str]) -> usize {
    let mut n = 0;
    for kw in keywords {
        for (idx, _) in corpus.match_indices(kw) {
            let mut start = idx.saturating_sub(16);
            while !corpus.is_char_boundary(start) {
                start -= 1;
            }
            let window = &corpus[start..idx];
            if NEGATORS.iter().any(|neg| window.contains(neg)) {
                continue;
            }
            n += 1;
        }
    }
    n
}

fn first_span<'a>(case: &'a IntakeCase, needle: &str) -> Option<&'a str> {
    for text in [
        &case.org_name,
        &case.about,
        &case.audience,
        &case.goal,
        &case.notes,
    ]
    .into_iter()
    .chain(case.services.iter())
    .chain(case.features.iter())
    {
        if let Some(pos) = text.to_lowercase().find(needle) {
            let end = (pos + needle.len()).min(text.len());
            // Snap to char boundaries.
            let mut s = pos;
            while !text.is_char_boundary(s) {
                s -= 1;
            }
            let mut e = end;
            while !text.is_char_boundary(e) {
                e += 1;
            }
            return Some(&text[s..e]);
        }
    }
    None
}

impl Classifier for DeterministicBaseline {
    fn name(&self) -> &str {
        "deterministic-baseline"
    }

    fn classify(&self, case: &IntakeCase, skins: &[&str]) -> Classification {
        let corpus = case.corpus();
        let word_count = corpus.split_whitespace().count();
        let head = {
            let mut h = String::new();
            for part in [&case.goal, &case.audience] {
                h.push_str(part);
                h.push(' ');
            }
            for v in &case.services {
                h.push_str(v);
                h.push(' ');
            }
            h.to_lowercase()
        };

        // Industry: highest positive vote count; ties or zero votes abstain
        // unless there is enough text to call it "other".
        let mut best: Option<(&str, usize, &str)> = None;
        let mut tie = false;
        for (industry, keywords) in INDUSTRY_KEYWORDS {
            // Body votes count once per distinct keyword; structured fields
            // (name, goal, audience, services) count double. This keeps a
            // stray mention in a long paragraph from outvoting the business.
            let body = keywords.iter().filter(|k| votes(&corpus, &[k]) > 0).count();
            let v = body + 2 * votes(&head, keywords);
            if v == 0 {
                continue;
            }
            match best {
                Some((_, bv, _)) if v == bv => tie = true,
                Some((_, bv, _)) if v < bv => {}
                _ => {
                    tie = false;
                    let kw = keywords
                        .iter()
                        .find(|k| votes(&corpus, &[k]) > 0)
                        .copied()
                        .unwrap_or("");
                    best = Some((industry, v, kw));
                }
            }
        }

        let undecided = [
            "not settled",
            "deciding between",
            "has not decided",
            "not decided",
            "undecided",
            "no business details",
        ]
        .iter()
        .any(|k| corpus.contains(k));
        let mut out = Classification::default();
        if undecided {
            out.abstain = true;
            return out;
        }
        let (industry, kw) = match best {
            Some((i, _, kw)) if !tie => (i, kw),
            _ if word_count >= 20 => ("other", ""),
            _ => {
                out.abstain = true;
                return out;
            }
        };
        out.industry = industry.to_string();
        if !kw.is_empty()
            && let Some(span) = first_span(case, kw)
        {
            out.grounding.insert("industry".into(), span.to_string());
        }

        // Task kind: interactive features push toward hybrid; explicit
        // proposal/quote language toward proposal; default brochure.
        let feature_text = case.features.join(" ").to_lowercase();
        let static_declared = [
            "static site",
            "static website",
            "no ordering",
            "no login",
            "no interactive",
            "no booking",
            "no portal",
            "informational website",
            "brochure site",
        ]
        .iter()
        .any(|k| corpus.contains(k));
        let interactive_terms = [
            "login",
            "portal",
            "booking",
            "scheduling",
            "cart",
            "search",
            " api",
            "cms",
            "content editor",
            "bulletin editor",
            "editor for",
            "journal",
            "bulletin",
            "ordering",
            "contact form",
            "booking form",
            "upload",
            "calendar",
            "reservation",
        ];
        // Explicitly listed features always count. Prose mentions count only
        // when not negated and no static declaration exists.
        let listed_interactive = interactive_terms.iter().any(|k| feature_text.contains(k));
        let about_l = case.about.to_lowercase();
        let notes_l = case.notes.to_lowercase();
        let prose_interactive = !static_declared
            && interactive_terms
                .iter()
                .any(|k| votes(&about_l, &[k]) > 0 || votes(&notes_l, &[k]) > 0);
        let interactive = listed_interactive || prose_interactive;
        let proposal = [
            "proposal",
            "replace its",
            "replace our",
            "replace the site",
            "replace an old",
            "replace a broken",
            "site replacement",
            "redesign",
            "quote request",
            "estimate request",
            "rebuild the site",
        ]
        .iter()
        .any(|k| corpus.contains(k));
        out.task_kind = if interactive {
            "hybrid"
        } else if proposal {
            "proposal"
        } else {
            "brochure"
        }
        .to_string();

        // Complexity: languages, feature count and text volume.
        let score = case.languages.len().saturating_sub(1) * 2
            + case.features.len()
            + usize::from(interactive) * 2
            + usize::from(word_count > 120);
        out.complexity = match score {
            0..=1 => "low",
            2..=4 => "medium",
            _ => "high",
        }
        .to_string();

        // Skins: rule table keyed by industry family; never more than two.
        let preferred: &[&str] = match industry {
            "auto-repair" | "construction" | "cleaning" | "fisheries" => &["workshop-index"],
            "religious-org" | "law" | "medical" | "insurance" => &["editorial-record"],
            _ => &["workshop-index", "editorial-record"],
        };
        out.skin_candidates = preferred
            .iter()
            .filter(|s| skins.contains(s))
            .map(|s| s.to_string())
            .take(2)
            .collect();
        out
    }
}

// ---------------------------------------------------------------------------
// Scoring, shared by every arm
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Scorecard {
    pub arm: String,
    pub cases: usize,
    pub invalid_outputs: usize,
    pub industry_correct: usize,
    pub task_kind_correct: usize,
    pub complexity_correct: usize,
    /// Cases where the gold says abstain and the arm abstained.
    pub abstain_correct: usize,
    pub abstain_expected: usize,
    /// Cases where the arm abstained but gold had an answer.
    pub over_abstained: usize,
    /// Any forbidden skin id emitted, or an unknown id: critical failure.
    pub critical_failures: usize,
    /// At least one acceptable skin present when gold lists any.
    pub skin_recall_hits: usize,
    pub skin_recall_denominator: usize,
    pub grounded_fields: usize,
    pub per_case: Vec<CaseResult>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    pub valid: bool,
    pub industry_ok: bool,
    pub task_kind_ok: bool,
    pub complexity_ok: bool,
    pub abstain_ok: Option<bool>,
    pub critical: bool,
    pub output: Classification,
}

pub fn score(
    arm: &dyn Classifier,
    cases: &[IntakeCase],
    labels: &[Label],
    skins: &[&str],
) -> Scorecard {
    let by_id: BTreeMap<&str, &Label> = labels.iter().map(|l| (l.id.as_str(), l)).collect();
    let mut sc = Scorecard {
        arm: arm.name().to_string(),
        ..Default::default()
    };
    for case in cases {
        let Some(label) = by_id.get(case.id.as_str()) else {
            continue;
        };
        sc.cases += 1;
        let out = arm.classify(case, skins);
        let valid = validate(case, &out, skins).is_ok();
        let forbidden_hit = out
            .skin_candidates
            .iter()
            .any(|s| label.forbidden_skins.iter().any(|f| f == s));
        let critical = !valid && !out.abstain || forbidden_hit;
        let expect_abstain = label.industry.is_none();
        let mut r = CaseResult {
            id: case.id.clone(),
            valid,
            critical,
            output: out.clone(),
            ..Default::default()
        };
        if !valid {
            sc.invalid_outputs += 1;
        }
        if critical {
            sc.critical_failures += 1;
        }
        if expect_abstain {
            sc.abstain_expected += 1;
            r.abstain_ok = Some(out.abstain);
            if out.abstain {
                sc.abstain_correct += 1;
            }
        } else if out.abstain {
            sc.over_abstained += 1;
        } else if valid {
            r.industry_ok = label.industry.as_deref() == Some(out.industry.as_str());
            r.task_kind_ok = label.task_kind.as_deref() == Some(out.task_kind.as_str());
            r.complexity_ok = label.complexity.as_deref() == Some(out.complexity.as_str());
            sc.industry_correct += usize::from(r.industry_ok);
            sc.task_kind_correct += usize::from(r.task_kind_ok);
            sc.complexity_correct += usize::from(r.complexity_ok);
            if !label.acceptable_skins.is_empty() {
                sc.skin_recall_denominator += 1;
                if out
                    .skin_candidates
                    .iter()
                    .any(|s| label.acceptable_skins.iter().any(|a| a == s))
                {
                    sc.skin_recall_hits += 1;
                }
            }
            sc.grounded_fields += out.grounding.len();
        }
        sc.per_case.push(r);
    }
    sc
}

/// Load the bundled fixture corpus (synthetic only).
pub fn bundled_fixtures() -> Result<(Vec<IntakeCase>, Vec<Label>), serde_json::Error> {
    let cases: Vec<IntakeCase> = serde_json::from_str(include_str!("../fixtures/cases.json"))?;
    let labels: Vec<Label> = serde_json::from_str(include_str!("../fixtures/labels.dev.json"))?;
    Ok((cases, labels))
}

#[cfg(test)]
mod tests;
