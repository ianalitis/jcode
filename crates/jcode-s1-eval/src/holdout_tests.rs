//! The durable gate on the decision holdout.
//!
//! The same rules are pre-checked with `scripts/check_decision_holdout.py` while a
//! holdout is being authored, so the author gets precise repair instructions. These
//! tests are what keeps them true afterwards: a holdout that stops satisfying the
//! protocol fails CI rather than quietly changing what a scorecard means.
//!
//! Why the upper bound on `forbidden` matters: `critical` counts a case where the
//! arm named a forbidden option. If most cases forbade an option, `critical` would
//! collapse into `wrong` and the guardrail question - "did the arm name something a
//! rule prohibits?" - would stop being measurable. The dev set forbids one option in
//! one of ten cases; the bound keeps the holdout in the same regime.

use super::*;

/// The most cases that may forbid an option, as a share of the holdout.
const MAX_FORBIDDEN_CASES: usize = 8;
/// The protocol's floor.
const MIN_FORBIDDEN_CASES: usize = 3;
const MIN_CASES: usize = 30;
const MAX_SCORE_BAND: f64 = 0.4;

fn holdout() -> Vec<DecisionFixture> {
    decision_holdout_fixtures().expect("the frozen holdout parses")
}

fn dev() -> Vec<DecisionFixture> {
    bundled_decision_fixtures().expect("the dev set parses")
}

fn state_text(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| format!("<unrenderable: {error}>"))
}

#[test]
fn the_holdout_is_frozen_at_thirty_or_more_cases() {
    let fixtures = holdout();
    assert!(
        fixtures.len() >= MIN_CASES,
        "the protocol requires at least {MIN_CASES} cases, found {}",
        fixtures.len()
    );
}

#[test]
fn every_holdout_request_satisfies_the_contract() {
    for fixture in holdout() {
        validate_request(&fixture.request).unwrap_or_else(|error| {
            panic!(
                "holdout case {} is not a valid request: {error}",
                fixture.request.id
            )
        });
    }
}

#[test]
fn the_holdout_keeps_the_protocol_mix() {
    let fixtures = holdout();
    let mut choice = 0;
    let mut noul = 0;
    let mut score = 0;
    let mut abstention_correct = 0;
    let mut forbidden_cases = 0;
    for fixture in &fixtures {
        match fixture.request.kind {
            DecisionKind::Choice => choice += 1,
            DecisionKind::Noul => noul += 1,
            DecisionKind::Score => score += 1,
        }
        if fixture.acceptable.is_empty() && fixture.acceptable_levels.is_none() {
            abstention_correct += 1;
        }
        if !fixture.forbidden.is_empty() {
            forbidden_cases += 1;
        }
    }
    assert!(
        choice >= 10,
        "holdout needs at least 10 choice cases, found {choice}"
    );
    assert!(
        noul >= 8,
        "holdout needs at least 8 noul cases, found {noul}"
    );
    assert!(
        score >= 8,
        "holdout needs at least 8 score cases, found {score}"
    );
    assert!(
        abstention_correct >= 3,
        "holdout needs at least 3 abstention-correct cases, found {abstention_correct}"
    );
    assert!(
        forbidden_cases >= MIN_FORBIDDEN_CASES,
        "holdout needs at least {MIN_FORBIDDEN_CASES} cases with a forbidden option, found {forbidden_cases}"
    );
    assert!(
        forbidden_cases <= MAX_FORBIDDEN_CASES,
        "{forbidden_cases} of {} cases forbid an option, so `critical` would stop being \
         distinguishable from `wrong`; keep it to {MAX_FORBIDDEN_CASES}",
        fixtures.len()
    );
}

#[test]
fn holdout_answer_keys_are_well_formed() {
    for fixture in holdout() {
        let ids: Vec<&str> = fixture
            .request
            .options
            .iter()
            .map(|o| o.id.as_str())
            .collect();
        for needed in fixture.acceptable.iter().chain(fixture.forbidden.iter()) {
            assert!(
                ids.contains(&needed.as_str()),
                "case {} answers with {needed:?}, which is not one of its options",
                fixture.request.id
            );
        }
        for accepted in &fixture.acceptable {
            assert!(
                !fixture.forbidden.contains(accepted),
                "case {} has {accepted:?} both acceptable and forbidden",
                fixture.request.id
            );
        }
        match fixture.request.kind {
            DecisionKind::Score => {
                assert!(
                    fixture.acceptable.is_empty(),
                    "score case {} must not carry option answers",
                    fixture.request.id
                );
                let [low, high] = fixture.acceptable_levels.unwrap_or_else(|| {
                    panic!("score case {} has no acceptable_levels", fixture.request.id)
                });
                // A band may end exactly at 1.0 (the top level) and start exactly at
                // 0.0 (the bottom one); only an inverted or empty band is wrong.
                assert!(
                    (0.0..=1.0).contains(&low) && (0.0..=1.0).contains(&high) && low < high,
                    "score case {} band {low}..{high} is out of range",
                    fixture.request.id
                );
                assert!(
                    high - low <= MAX_SCORE_BAND,
                    "score case {} band {low}..{high} is wider than {MAX_SCORE_BAND}, so it cannot discriminate",
                    fixture.request.id
                );
            }
            DecisionKind::Noul => {
                assert_eq!(
                    fixture.request.options.len(),
                    2,
                    "noul case {} needs exactly two options",
                    fixture.request.id
                );
                assert!(
                    fixture.acceptable_levels.is_none(),
                    "noul case {} must not carry acceptable_levels",
                    fixture.request.id
                );
            }
            DecisionKind::Choice => {
                assert!(
                    fixture.acceptable_levels.is_none(),
                    "choice case {} must not carry acceptable_levels",
                    fixture.request.id
                );
            }
        }
    }
}

#[test]
fn no_holdout_case_reuses_a_dev_state_or_option_set() {
    let dev_fixtures = dev();
    let dev_states: Vec<String> = dev_fixtures
        .iter()
        .map(|fixture| state_text(&fixture.request.state))
        .collect();
    let dev_option_sets: Vec<Vec<String>> = dev_fixtures
        .iter()
        .map(|fixture| {
            let mut ids: Vec<String> = fixture
                .request
                .options
                .iter()
                .map(|option| option.id.clone())
                .collect();
            ids.sort_unstable();
            ids
        })
        .collect();

    let mut seen_states: Vec<(String, String)> = Vec::new();
    for fixture in holdout() {
        let state = state_text(&fixture.request.state);
        assert!(
            !dev_states.contains(&state),
            "holdout case {} reuses a dev state string",
            fixture.request.id
        );
        if let Some((other, _)) = seen_states.iter().find(|(text, _)| text == &state) {
            panic!(
                "holdout cases {} and {other} share a state string",
                fixture.request.id
            );
        }
        seen_states.push((state, fixture.request.id.clone()));

        let mut ids: Vec<String> = fixture
            .request
            .options
            .iter()
            .map(|option| option.id.clone())
            .collect();
        ids.sort_unstable();
        assert!(
            !dev_option_sets.contains(&ids),
            "holdout case {} reuses a dev option-id set {ids:?}",
            fixture.request.id
        );
    }
}

#[test]
fn the_holdout_digest_is_stable_and_quoted_by_receipts() {
    // Two calls, one value: the digest identifies embedded bytes, not a file that
    // could change under it.
    assert_eq!(decision_holdout_sha256(), decision_holdout_sha256());
    assert_eq!(decision_holdout_sha256().len(), 64);
    assert!(decision_holdout_json().trim_start().starts_with('['));
}
