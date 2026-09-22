//! The typed decision contract: `Noul` / `Choice` / `Score`, `authority: advisory`.
//!
//! This is the model-free half of the decision work (W1 and W5 in
//! `~/dotfiles/docs/research/2026-09-21-semantic-if-routing-and-harness-insights.md`
//! §2 and §9). It is a *contract*, not a subsystem: no network, no shell, no
//! model call, and no new state. Arms plug in through [`DecisionArm`] and are
//! validated by the same [`validate`] seam as the classifier next door.
//!
//! Three properties are enforced here rather than trusted:
//!
//! 1. **Typed, not free text.** A `Noul` cannot return prose, and a `Choice`
//!    cannot return an option that was not supplied. The options are a closed
//!    set of 2 to 16 described ids, so the parse-and-repair step disappears.
//! 2. **`authority: advisory` is the only admitted value.** A decision may
//!    inform deterministic policy and may never grant a permission, widen a data
//!    class, authorise spend, select a provider, or trigger an effect. The type
//!    carries the rule so a receipt can quote it instead of restating it.
//! 3. **A probability is not a gate.** Probabilities may be reported freely;
//!    gating on one (a declared [`DecisionRequest::threshold`]) requires a
//!    [`DecisionRequest::calibration_ref`] other than `"uncalibrated"`. This is
//!    W5, enforced at the schema rather than by convention.
//!
//! Abstention is a first-class outcome, exactly as `Classification::abstain` is
//! for the classifier: an abstaining result is valid and is never an error.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// The closed option set is 2 to 16 entries, matching the research note's cap
/// and staying well inside the transport's 2 to 255 criteria bound.
pub const MIN_OPTIONS: usize = 2;
pub const MAX_OPTIONS: usize = 16;
/// The only admitted `authority` value today.
pub const ADVISORY_AUTHORITY: &str = "advisory";
/// The literal that means "no fitted calibration applies".
pub const UNCALIBRATED: &str = "uncalibrated";

fn advisory() -> String {
    ADVISORY_AUTHORITY.to_string()
}

fn uncalibrated() -> String {
    UNCALIBRATED.to_string()
}

/// The three decision shapes. There is no fourth, and no free-text variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionKind {
    /// Yes or no.
    Noul,
    /// One of the supplied options.
    Choice,
    /// A level on a scale.
    Score,
}

impl DecisionKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Noul => "noul",
            Self::Choice => "choice",
            Self::Score => "score",
        }
    }
}

/// One member of the closed option set. Code owns the set; no arm may extend it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionOption {
    pub id: String,
    #[serde(default)]
    pub description: String,
}

/// The question. Every field here is bound into [`DecisionRequest::prompt_sha256`]
/// so a receipt can prove which question produced a result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRequest {
    /// Stable id, bound into the receipt.
    pub id: String,
    pub kind: DecisionKind,
    /// The text or structured JSON to judge. Never uploaded without an
    /// eligibility pass; this contract does not decide that.
    #[serde(default)]
    pub state: serde_json::Value,
    /// The runtime natural-language question.
    #[serde(default)]
    pub criterion: String,
    /// Closed set, 2 to 16 described options, in a meaningful order.
    #[serde(default)]
    pub options: Vec<DecisionOption>,
    /// Only [`ADVISORY_AUTHORITY`] is admitted; see the module docs.
    #[serde(default = "advisory")]
    pub authority: String,
    /// Present only when the caller gates on a probability. Gating requires a
    /// calibration reference; reporting does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    /// Which fitted temperature applies, or [`UNCALIBRATED`].
    #[serde(default = "uncalibrated")]
    pub calibration_ref: String,
    /// Binds `criterion`, the option ids and descriptions, their order, and the
    /// kind. Set it with [`DecisionRequest::bind`] rather than by hand.
    #[serde(default)]
    pub prompt_sha256: String,
}

impl DecisionRequest {
    /// The exact bytes `prompt_sha256` covers. Order matters: reordering the
    /// options changes the digest, which is the point.
    pub fn prompt_preimage(&self) -> String {
        let mut s = String::new();
        s.push_str(self.kind.name());
        s.push('\u{1f}');
        s.push_str(self.criterion.trim());
        s.push('\u{1f}');
        for option in &self.options {
            s.push_str(option.id.trim());
            s.push('\u{1e}');
            s.push_str(option.description.trim());
            s.push('\u{1f}');
        }
        s
    }

    /// `prompt_sha256` for the current fields, lowercase hex.
    pub fn computed_prompt_sha256(&self) -> String {
        hex_lower(&Sha256::digest(self.prompt_preimage().as_bytes()))
    }

    /// Set `prompt_sha256` from the current fields.
    pub fn bind(&mut self) {
        self.prompt_sha256 = self.computed_prompt_sha256();
    }

    /// True when a fitted calibration applies. `"uncalibrated"` is the sentinel,
    /// so an empty reference is uncalibrated too.
    pub fn is_calibrated(&self) -> bool {
        let reference = self.calibration_ref.trim();
        !reference.is_empty() && !reference.eq_ignore_ascii_case(UNCALIBRATED)
    }

    /// True when the caller intends to compare a probability against
    /// [`Self::threshold`].
    pub fn gates_on_probability(&self) -> bool {
        self.threshold.is_some()
    }
}

/// One option's probability, conditional on the supplied option set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionScore {
    pub option_id: String,
    pub probability: f64,
}

/// The answer. Exactly one of `noul` / `choice` / `score` is meaningful for a
/// non-abstaining result, selected by the request's kind; `scores` is the
/// optional distribution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DecisionResult {
    #[serde(default)]
    pub request_id: String,
    /// First-class outcome: a valid answer that refuses to choose.
    #[serde(default)]
    pub abstain: bool,
    /// `Noul` answers: the probability of yes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noul: Option<f64>,
    /// `Choice` answers: the selected option id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<String>,
    /// `Score` answers: the level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default)]
    pub scores: Vec<OptionScore>,
    /// Which arm produced this: `local_semif(model, revision, quant)`,
    /// `cloud_jev(model_id)`, or a baseline name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    /// Field name to a literal span of `state` that supports the value.
    #[serde(default)]
    pub grounding: BTreeMap<String, String>,
    /// Must equal the request's binding, so a result cannot be replayed against
    /// a different question.
    #[serde(default)]
    pub prompt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecisionError {
    /// The request's `authority` is not `advisory`.
    UnadmittedAuthority(String),
    /// Fewer than [`MIN_OPTIONS`] options were supplied.
    TooFewOptions(usize),
    /// More than [`MAX_OPTIONS`] options were supplied.
    TooManyOptions(usize),
    /// Two options share an id.
    DuplicateOption(String),
    /// An option id or description is blank.
    BlankOption(String),
    /// `prompt_sha256` is missing or does not match the bound fields.
    UnboundPrompt,
    /// A `threshold` is not a finite number in 0.0..=1.0.
    InvalidThreshold(f64),
    /// A threshold gates on a probability with no fitted calibration.
    UncalibratedThreshold(f64),
    /// The result answers a different request.
    RequestMismatch(String),
    /// The result's binding does not match the request's.
    ResultBindingMismatch,
    /// A result names an option that was not supplied.
    UnknownOption(String),
    /// The result's answer field does not match the request's kind.
    AnswerKindMismatch(DecisionKind),
    /// A probability is not a finite number in 0.0..=1.0.
    ProbabilityOutOfRange(f64),
    /// A grounding span is empty or is not a literal substring of `state`.
    UngroundedField(String),
    /// `state` cannot be rendered as text, so a grounding span cannot be checked
    /// against it. Reported as its own cause rather than as every span failing.
    UnrenderableState(String),
}

impl std::fmt::Display for DecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnadmittedAuthority(value) => write!(
                f,
                "decision authority must be {ADVISORY_AUTHORITY}, got {value:?}"
            ),
            Self::TooFewOptions(n) => write!(
                f,
                "decisions require at least {MIN_OPTIONS} options, got {n}"
            ),
            Self::TooManyOptions(n) => {
                write!(f, "decisions allow at most {MAX_OPTIONS} options, got {n}")
            }
            Self::DuplicateOption(id) => write!(f, "duplicate decision option id {id:?}"),
            Self::BlankOption(id) => {
                write!(f, "decision option {id:?} needs an id and a description")
            }
            Self::UnboundPrompt => write!(
                f,
                "decision request is not bound to its criterion and options"
            ),
            Self::InvalidThreshold(t) => {
                write!(f, "decision threshold {t} must be within 0.0..=1.0")
            }
            Self::UncalibratedThreshold(t) => write!(
                f,
                "decision gates on probability {t} without a calibration reference"
            ),
            Self::RequestMismatch(id) => {
                write!(f, "decision result does not answer request {id:?}")
            }
            Self::ResultBindingMismatch => {
                write!(f, "decision result binding does not match the request")
            }
            Self::UnknownOption(id) => write!(f, "unknown decision option id {id:?}"),
            Self::AnswerKindMismatch(kind) => {
                write!(f, "decision result carries no {} answer", kind.name())
            }
            Self::ProbabilityOutOfRange(p) => write!(f, "probability {p} must be within 0.0..=1.0"),
            Self::UngroundedField(field) => write!(
                f,
                "decision grounding for {field:?} is not a span of the state"
            ),
            Self::UnrenderableState(detail) => write!(
                f,
                "decision state cannot be rendered for grounding: {detail}"
            ),
        }
    }
}

impl std::error::Error for DecisionError {}

/// The request half of the seam. Call it before spending a call on an arm.
pub fn validate_request(request: &DecisionRequest) -> Result<(), DecisionError> {
    if request.authority != ADVISORY_AUTHORITY {
        return Err(DecisionError::UnadmittedAuthority(
            request.authority.clone(),
        ));
    }
    if request.options.len() < MIN_OPTIONS {
        return Err(DecisionError::TooFewOptions(request.options.len()));
    }
    if request.options.len() > MAX_OPTIONS {
        return Err(DecisionError::TooManyOptions(request.options.len()));
    }
    let mut seen = BTreeSet::new();
    for option in &request.options {
        if option.id.trim().is_empty() || option.description.trim().is_empty() {
            return Err(DecisionError::BlankOption(option.id.clone()));
        }
        if !seen.insert(option.id.as_str()) {
            return Err(DecisionError::DuplicateOption(option.id.clone()));
        }
    }
    if request.prompt_sha256.is_empty() || request.prompt_sha256 != request.computed_prompt_sha256()
    {
        return Err(DecisionError::UnboundPrompt);
    }
    if let Some(threshold) = request.threshold {
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(DecisionError::InvalidThreshold(threshold));
        }
        if !request.is_calibrated() {
            return Err(DecisionError::UncalibratedThreshold(threshold));
        }
    }
    Ok(())
}

/// The whole seam: the request, and the result an arm produced for it. This is
/// the function the packet's acceptance refers to when it says an unknown option
/// id is rejected by `validate()`.
pub fn validate(request: &DecisionRequest, result: &DecisionResult) -> Result<(), DecisionError> {
    validate_request(request)?;
    if result.request_id != request.id {
        return Err(DecisionError::RequestMismatch(result.request_id.clone()));
    }
    if result.prompt_sha256 != request.prompt_sha256 {
        return Err(DecisionError::ResultBindingMismatch);
    }
    if result.abstain {
        return Ok(());
    }
    match request.kind {
        DecisionKind::Noul if result.noul.is_none() => {
            return Err(DecisionError::AnswerKindMismatch(request.kind));
        }
        DecisionKind::Choice if result.choice.is_none() => {
            return Err(DecisionError::AnswerKindMismatch(request.kind));
        }
        DecisionKind::Score if result.score.is_none() => {
            return Err(DecisionError::AnswerKindMismatch(request.kind));
        }
        _ => {}
    }
    if let Some(probability) = result.noul {
        check_probability(probability)?;
    }
    if let Some(level) = result.score {
        check_probability(level)?;
    }
    if let Some(id) = result.choice.as_deref()
        && !request.options.iter().any(|option| option.id == id)
    {
        return Err(DecisionError::UnknownOption(id.to_string()));
    }
    let mut scored = BTreeSet::new();
    for entry in &result.scores {
        check_probability(entry.probability)?;
        if !request
            .options
            .iter()
            .any(|option| option.id == entry.option_id)
        {
            return Err(DecisionError::UnknownOption(entry.option_id.clone()));
        }
        if !scored.insert(entry.option_id.as_str()) {
            return Err(DecisionError::DuplicateOption(entry.option_id.clone()));
        }
    }
    // Render the state only when there is a span to check it against, so an arm
    // that grounds nothing is never failed for an unrelated reason. The state is
    // rendered with `?` rather than defaulted to empty: an empty haystack would
    // report every span as ungrounded, which hides the real cause.
    if !result.grounding.is_empty() {
        let haystack = serde_json::to_string(&request.state)
            .map_err(|error| DecisionError::UnrenderableState(error.to_string()))?;
        for (field, span) in &result.grounding {
            if !super::is_grounded(&haystack, span) {
                return Err(DecisionError::UngroundedField(field.clone()));
            }
        }
    }
    Ok(())
}

fn check_probability(value: f64) -> Result<(), DecisionError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(DecisionError::ProbabilityOutOfRange(value));
    }
    Ok(())
}

/// Any arm behind the contract, including the deterministic baseline.
pub trait DecisionArm {
    fn name(&self) -> &str;
    fn decide(&self, request: &DecisionRequest) -> DecisionResult;
}

/// Rule baseline: no model, no network. It exists to prove the contract end to
/// end, not to be good.
///
/// `Choice` votes for the option whose description shares the most content words
/// with the state, and abstains on a tie or no overlap. `Noul` answers yes when
/// the criterion's content words appear in the state. `Score` is the share of
/// the option vocabulary the state covers.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicDecisionBaseline;

impl DecisionArm for DeterministicDecisionBaseline {
    fn name(&self) -> &str {
        "deterministic-decision-baseline"
    }

    fn decide(&self, request: &DecisionRequest) -> DecisionResult {
        let mut result = DecisionResult {
            request_id: request.id.clone(),
            prompt_sha256: request.prompt_sha256.clone(),
            backend: Some(self.name().to_string()),
            ..Default::default()
        };
        // A `Value` whose maps have non-string keys is the only way this fails,
        // and it means the state cannot be read as text at all. The baseline
        // abstains rather than judging an empty string, which would look like a
        // confident "no overlap" answer.
        let state = match serde_json::to_string(&request.state) {
            Ok(rendered) => content_words(&rendered),
            Err(error) => {
                debug_assert!(false, "the decision state did not serialise: {error}");
                return DecisionResult {
                    request_id: request.id.clone(),
                    prompt_sha256: request.prompt_sha256.clone(),
                    abstain: true,
                    backend: Some(self.name().to_string()),
                    ..Default::default()
                };
            }
        };
        match request.kind {
            DecisionKind::Noul => {
                let criterion = content_words(&request.criterion);
                let hits = criterion.intersection(&state).count();
                if criterion.is_empty() {
                    result.abstain = true;
                    return result;
                }
                result.noul = Some(hits as f64 / criterion.len() as f64);
            }
            DecisionKind::Choice => {
                let mut best: Option<(&str, usize)> = None;
                let mut tie = false;
                for option in &request.options {
                    let words = content_words(&option.description);
                    let hits = words.intersection(&state).count();
                    match best {
                        Some((_, best_hits)) if hits == best_hits && hits > 0 => tie = true,
                        Some((_, best_hits)) if hits <= best_hits => {}
                        _ => {
                            tie = false;
                            best = Some((option.id.as_str(), hits));
                        }
                    }
                }
                match best {
                    Some((id, hits)) if hits > 0 && !tie => {
                        result.choice = Some(id.to_string());
                    }
                    _ => result.abstain = true,
                }
            }
            DecisionKind::Score => {
                let vocabulary: BTreeSet<String> = request
                    .options
                    .iter()
                    .flat_map(|option| content_words(&option.description))
                    .collect();
                if vocabulary.is_empty() {
                    result.abstain = true;
                    return result;
                }
                let covered = vocabulary.intersection(&state).count();
                result.score = Some(covered as f64 / vocabulary.len() as f64);
            }
        }
        result
    }
}

/// Lowercase alphanumeric words of three or more characters, minus the handful
/// of English function words that carry no signal. Deterministic and locale-free
/// on purpose: this is a baseline, not a tokenizer.
fn content_words(text: &str) -> BTreeSet<String> {
    const STOPWORDS: &[&str] = &[
        "and", "are", "but", "for", "from", "has", "have", "into", "its", "not", "that", "the",
        "their", "then", "there", "these", "this", "was", "were", "with", "you", "your",
    ];
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() >= 3)
        .map(|word| word.to_lowercase())
        .filter(|word| !STOPWORDS.contains(&word.as_str()))
        .collect()
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// One fixture case for an arm: the request, and what a correct answer looks
/// like. Synthetic only; never a real client or a private state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionFixture {
    pub request: DecisionRequest,
    /// Option ids that answer the case correctly. Empty means abstention is the
    /// correct answer. For `Noul`, the convention is that the request declares
    /// `yes` and `no` and the arm's answer is `yes` when `noul >= 0.5`.
    #[serde(default)]
    pub acceptable: Vec<String>,
    /// Option ids that must never appear.
    #[serde(default)]
    pub forbidden: Vec<String>,
    /// For `Score`: the inclusive level range that answers the case correctly.
    #[serde(default)]
    pub acceptable_levels: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DecisionCaseResult {
    pub id: String,
    pub valid: bool,
    pub abstained: bool,
    pub correct: bool,
    pub critical: bool,
    pub result: DecisionResult,
}

/// What an arm did over a fixture set. Contract violations are counted
/// separately from quality: an arm that answers the wrong option is a wrong
/// answer, and an arm that names an option it was not given is invalid.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DecisionScorecard {
    pub arm: String,
    pub cases: usize,
    pub invalid: usize,
    pub abstained: usize,
    pub correct: usize,
    /// Cases where an option was acceptable and the arm abstained.
    pub over_abstained: usize,
    /// A forbidden option was named, or a `Score` case has no usable level.
    pub critical: usize,
    pub per_case: Vec<DecisionCaseResult>,
}

/// Run one arm over the fixture set and score every result through the contract.
pub fn score_decisions(arm: &dyn DecisionArm, fixtures: &[DecisionFixture]) -> DecisionScorecard {
    let mut card = DecisionScorecard {
        arm: arm.name().to_string(),
        ..Default::default()
    };
    for fixture in fixtures {
        let request = &fixture.request;
        let result = arm.decide(request);
        let valid = validate(request, &result).is_ok();
        let answer = answer_id(request, &result);
        let forbidden_hit = answer
            .as_deref()
            .is_some_and(|id| fixture.forbidden.iter().any(|f| f == id));
        let critical = !valid || forbidden_hit;
        let correct =
            valid && !result.abstain && is_correct(fixture, request, &result, answer.as_deref());
        card.cases += 1;
        card.invalid += usize::from(!valid);
        card.abstained += usize::from(result.abstain);
        card.correct += usize::from(correct);
        card.critical += usize::from(critical);
        if result.abstain && !fixture.acceptable.is_empty() {
            card.over_abstained += 1;
        }
        card.per_case.push(DecisionCaseResult {
            id: request.id.clone(),
            valid,
            abstained: result.abstain,
            correct,
            critical,
            result,
        });
    }
    card
}

/// The option id an answer amounts to, for `Choice` and `Noul`.
fn answer_id(request: &DecisionRequest, result: &DecisionResult) -> Option<String> {
    match request.kind {
        DecisionKind::Choice => result.choice.clone(),
        DecisionKind::Noul => {
            let probability = result.noul?;
            let yes = request.options.first()?.id.clone();
            let no = request.options.get(1)?.id.clone();
            Some(if probability >= 0.5 { yes } else { no })
        }
        DecisionKind::Score => None,
    }
}

fn is_correct(
    fixture: &DecisionFixture,
    request: &DecisionRequest,
    result: &DecisionResult,
    answer: Option<&str>,
) -> bool {
    match request.kind {
        DecisionKind::Score => match (fixture.acceptable_levels, result.score) {
            (Some([low, high]), Some(level)) => (low..=high).contains(&level),
            // A Score case with no declared range is not scored for quality.
            _ => false,
        },
        _ => answer.is_some_and(|id| fixture.acceptable.iter().any(|a| a == id)),
    }
}

/// The bundled dev set's exact text, for a receipt that has to name it.
pub fn bundled_decision_fixtures_json() -> &'static str {
    include_str!("../fixtures/decisions.dev.json")
}

/// `sha256` of [`bundled_decision_fixtures_json`], lowercase hex.
pub fn bundled_decision_fixtures_sha256() -> String {
    hex_lower(&Sha256::digest(bundled_decision_fixtures_json().as_bytes()))
}

/// Load the bundled decision fixtures (synthetic only). This is a *dev* set:
/// like `labels.dev.json` it measures fit, not generalisation, and a quality
/// claim needs a holdout authored by a session that has not read the arm.
pub fn bundled_decision_fixtures() -> Result<Vec<DecisionFixture>, serde_json::Error> {
    let mut fixtures: Vec<DecisionFixture> =
        serde_json::from_str(bundled_decision_fixtures_json())?;
    // The bundled file omits the binding, so editing a case cannot leave a stale
    // digest behind: the loader binds each request from the case it just read.
    for fixture in &mut fixtures {
        if fixture.request.prompt_sha256.is_empty() {
            fixture.request.bind();
        }
    }
    Ok(fixtures)
}

/// The frozen decision holdout's exact text.
///
/// Embedded rather than read at runtime so the digest below identifies the bytes
/// that were scored, and so a change to the holdout cannot pass unnoticed: it
/// changes the hash in every subsequent receipt.
pub fn decision_holdout_json() -> &'static str {
    include_str!("../fixtures/decisions.holdout.json")
}

/// `sha256` of [`decision_holdout_json`], lowercase hex. Receipts quote it to name
/// which fixture set they scored.
pub fn decision_holdout_sha256() -> String {
    hex_lower(&Sha256::digest(decision_holdout_json().as_bytes()))
}

/// Load the frozen decision holdout. It is a *holdout*: authored by a session that
/// had not read the arm or the baseline, answers frozen before any arm ran, and
/// scored once per arm per revision. See `fixtures/HOLDOUT-PROTOCOL.md`.
///
/// The bundled file leaves `prompt_sha256` empty so the loader binds it, exactly as
/// the dev loader does; a hand-edited digest therefore cannot go stale.
pub fn decision_holdout_fixtures() -> Result<Vec<DecisionFixture>, serde_json::Error> {
    let mut fixtures: Vec<DecisionFixture> = serde_json::from_str(decision_holdout_json())?;
    for fixture in &mut fixtures {
        if fixture.request.prompt_sha256.is_empty() {
            fixture.request.bind();
        }
    }
    Ok(fixtures)
}

#[cfg(test)]
#[path = "decision_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "holdout_tests.rs"]
mod holdout_tests;
