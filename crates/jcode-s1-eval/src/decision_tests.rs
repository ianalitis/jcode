//! Acceptance tests for the typed decision contract (W1 and W5).

use super::*;
use serde_json::json;

fn options() -> Vec<DecisionOption> {
    vec![
        DecisionOption {
            id: "click".into(),
            description: "Click the add to cart button".into(),
        },
        DecisionOption {
            id: "stop".into(),
            description: "Return control to the operator".into(),
        },
    ]
}

fn request(kind: DecisionKind) -> DecisionRequest {
    let mut request = DecisionRequest {
        id: "decision-1".into(),
        kind,
        state: json!({"page": "Add the item to the cart, then continue to checkout"}),
        criterion: "Which browser action moves the purchase forward?".into(),
        options: options(),
        authority: ADVISORY_AUTHORITY.into(),
        threshold: None,
        calibration_ref: UNCALIBRATED.into(),
        prompt_sha256: String::new(),
    };
    request.bind();
    request
}

fn result_for(request: &DecisionRequest) -> DecisionResult {
    DecisionResult {
        request_id: request.id.clone(),
        prompt_sha256: request.prompt_sha256.clone(),
        backend: Some("stub-arm".into()),
        ..Default::default()
    }
}

/// A stubbed arm: the second of the two the packet requires to round-trip.
struct StubArm;

impl DecisionArm for StubArm {
    fn name(&self) -> &str {
        "stub-arm"
    }

    fn decide(&self, request: &DecisionRequest) -> DecisionResult {
        let mut result = DecisionResult {
            request_id: request.id.clone(),
            prompt_sha256: request.prompt_sha256.clone(),
            backend: Some(self.name().into()),
            grounding: [("choice".to_string(), "cart".to_string())].into(),
            ..Default::default()
        };
        match request.kind {
            DecisionKind::Noul => result.noul = Some(0.7),
            DecisionKind::Choice => {
                result.choice = Some("click".into());
                result.scores = vec![
                    OptionScore {
                        option_id: "click".into(),
                        probability: 0.9,
                    },
                    OptionScore {
                        option_id: "stop".into(),
                        probability: 0.1,
                    },
                ];
            }
            DecisionKind::Score => result.score = Some(0.4),
        }
        result
    }
}

#[test]
fn request_binds_criterion_options_and_their_order() {
    let bound = request(DecisionKind::Choice);
    validate_request(&bound).unwrap();

    let mut other_criterion = bound.clone();
    other_criterion.criterion = "A different question".into();
    assert_eq!(
        validate_request(&other_criterion),
        Err(DecisionError::UnboundPrompt)
    );

    let mut reordered = bound.clone();
    reordered.options.swap(0, 1);
    assert_eq!(
        validate_request(&reordered),
        Err(DecisionError::UnboundPrompt),
        "the binding must cover option ordering"
    );

    let mut reworded = bound.clone();
    reworded.options[0].description = "Press the cart button".into();
    assert_eq!(
        validate_request(&reworded),
        Err(DecisionError::UnboundPrompt)
    );

    let mut other_kind = bound.clone();
    other_kind.kind = DecisionKind::Score;
    assert_eq!(
        validate_request(&other_kind),
        Err(DecisionError::UnboundPrompt)
    );

    let unbound = DecisionRequest {
        prompt_sha256: String::new(),
        ..bound
    };
    assert_eq!(
        validate_request(&unbound),
        Err(DecisionError::UnboundPrompt)
    );
}

#[test]
fn unknown_option_id_is_rejected_by_validate() {
    let request = request(DecisionKind::Choice);

    let mut unknown_choice = result_for(&request);
    unknown_choice.choice = Some("submit".into());
    assert_eq!(
        validate(&request, &unknown_choice),
        Err(DecisionError::UnknownOption("submit".into()))
    );

    let mut unknown_score = result_for(&request);
    unknown_score.choice = Some("click".into());
    unknown_score.scores = vec![OptionScore {
        option_id: "submit".into(),
        probability: 0.5,
    }];
    assert_eq!(
        validate(&request, &unknown_score),
        Err(DecisionError::UnknownOption("submit".into()))
    );

    let mut known = result_for(&request);
    known.choice = Some("click".into());
    validate(&request, &known).unwrap();
}

#[test]
fn gating_on_a_probability_without_calibration_is_refused() {
    let mut uncalibrated = request(DecisionKind::Score);
    uncalibrated.threshold = Some(0.8);
    uncalibrated.bind();
    assert_eq!(
        validate_request(&uncalibrated),
        Err(DecisionError::UncalibratedThreshold(0.8))
    );

    let mut blank_ref = request(DecisionKind::Noul);
    blank_ref.threshold = Some(0.5);
    blank_ref.calibration_ref = String::new();
    blank_ref.bind();
    assert_eq!(
        validate_request(&blank_ref),
        Err(DecisionError::UncalibratedThreshold(0.5))
    );

    let mut out_of_range = request(DecisionKind::Noul);
    out_of_range.threshold = Some(1.4);
    out_of_range.bind();
    assert_eq!(
        validate_request(&out_of_range),
        Err(DecisionError::InvalidThreshold(1.4))
    );

    // A fitted temperature is what makes the gate admissible, and reporting a
    // probability without a threshold stays free.
    let mut calibrated = request(DecisionKind::Score);
    calibrated.threshold = Some(0.8);
    calibrated.calibration_ref = "temperature-2026-09-22".into();
    calibrated.bind();
    validate_request(&calibrated).unwrap();

    let mut reported = result_for(&calibrated);
    reported.score = Some(0.62);
    validate(&calibrated, &reported).unwrap();
}

#[test]
fn both_arms_round_trip_a_request_through_serde() {
    for kind in [
        DecisionKind::Noul,
        DecisionKind::Choice,
        DecisionKind::Score,
    ] {
        let request = request(kind);
        let wire = serde_json::to_string(&request).unwrap();
        let parsed: DecisionRequest = serde_json::from_str(&wire).unwrap();
        assert_eq!(parsed, request);
        validate_request(&parsed).unwrap();

        for arm in [
            &DeterministicDecisionBaseline as &dyn DecisionArm,
            &StubArm as &dyn DecisionArm,
        ] {
            let result = arm.decide(&parsed);
            let wire = serde_json::to_string(&result).unwrap();
            let round_tripped: DecisionResult = serde_json::from_str(&wire).unwrap();
            assert_eq!(round_tripped, result);
            validate(&parsed, &round_tripped).unwrap_or_else(|error| {
                panic!("{} must satisfy the contract: {error}", arm.name())
            });
        }
    }
}

#[test]
fn option_set_is_closed_at_two_to_sixteen() {
    let mut one = request(DecisionKind::Choice);
    one.options.truncate(1);
    one.bind();
    assert_eq!(validate_request(&one), Err(DecisionError::TooFewOptions(1)));

    let mut many = request(DecisionKind::Choice);
    many.options = (0..17)
        .map(|i| DecisionOption {
            id: format!("o{i}"),
            description: format!("Option number {i}"),
        })
        .collect();
    many.bind();
    assert_eq!(
        validate_request(&many),
        Err(DecisionError::TooManyOptions(17))
    );

    let mut duplicated = request(DecisionKind::Choice);
    duplicated.options[1].id = "click".into();
    duplicated.bind();
    assert_eq!(
        validate_request(&duplicated),
        Err(DecisionError::DuplicateOption("click".into()))
    );

    let mut undescribed = request(DecisionKind::Choice);
    undescribed.options[0].description = "   ".into();
    undescribed.bind();
    assert_eq!(
        validate_request(&undescribed),
        Err(DecisionError::BlankOption("click".into()))
    );
}

#[test]
fn abstention_is_a_first_class_outcome() {
    for kind in [
        DecisionKind::Noul,
        DecisionKind::Choice,
        DecisionKind::Score,
    ] {
        let request = request(kind);
        let mut abstaining = result_for(&request);
        abstaining.abstain = true;
        validate(&request, &abstaining).unwrap();
    }
}

#[test]
fn authority_must_be_advisory() {
    let mut granting = request(DecisionKind::Choice);
    granting.authority = "granting".into();
    granting.bind();
    assert_eq!(
        validate_request(&granting),
        Err(DecisionError::UnadmittedAuthority("granting".into()))
    );

    // The field defaults to the only admitted value, so an arm cannot widen it
    // by omission.
    let defaulted: DecisionRequest = serde_json::from_value(json!({
        "id": "decision-2",
        "kind": "noul",
        "options": [
            {"id": "yes", "description": "Yes"},
            {"id": "no", "description": "No"}
        ]
    }))
    .unwrap();
    assert_eq!(defaulted.authority, ADVISORY_AUTHORITY);
    assert_eq!(defaulted.calibration_ref, UNCALIBRATED);
}

#[test]
fn probabilities_must_be_finite_and_within_range() {
    let request = request(DecisionKind::Noul);

    let mut above = result_for(&request);
    above.noul = Some(1.2);
    assert_eq!(
        validate(&request, &above),
        Err(DecisionError::ProbabilityOutOfRange(1.2))
    );

    let mut nan = result_for(&request);
    nan.noul = Some(f64::NAN);
    assert!(matches!(
        validate(&request, &nan),
        Err(DecisionError::ProbabilityOutOfRange(value)) if value.is_nan()
    ));

    let mut duplicate_scores = result_for(&request);
    duplicate_scores.noul = Some(0.5);
    duplicate_scores.scores = vec![
        OptionScore {
            option_id: "click".into(),
            probability: 0.5,
        },
        OptionScore {
            option_id: "click".into(),
            probability: 0.5,
        },
    ];
    assert_eq!(
        validate(&request, &duplicate_scores),
        Err(DecisionError::DuplicateOption("click".into()))
    );
}

#[test]
fn grounding_spans_must_be_literal_spans_of_the_state() {
    let request = request(DecisionKind::Choice);

    let mut grounded = result_for(&request);
    grounded.choice = Some("click".into());
    grounded.grounding = [("choice".to_string(), "Add the item to the cart".to_string())].into();
    validate(&request, &grounded).unwrap();

    let mut invented = result_for(&request);
    invented.choice = Some("click".into());
    invented.grounding = [("choice".to_string(), "the user already paid".to_string())].into();
    assert_eq!(
        validate(&request, &invented),
        Err(DecisionError::UngroundedField("choice".into()))
    );

    let mut blank = result_for(&request);
    blank.choice = Some("click".into());
    blank.grounding = [("choice".to_string(), "  ".to_string())].into();
    assert_eq!(
        validate(&request, &blank),
        Err(DecisionError::UngroundedField("choice".into()))
    );
}

#[test]
fn a_result_must_answer_its_request_with_the_declared_kind() {
    let request = request(DecisionKind::Choice);

    let mut wrong_kind = result_for(&request);
    wrong_kind.noul = Some(0.9);
    assert_eq!(
        validate(&request, &wrong_kind),
        Err(DecisionError::AnswerKindMismatch(DecisionKind::Choice))
    );

    let mut other_request = result_for(&request);
    other_request.request_id = "decision-99".into();
    assert_eq!(
        validate(&request, &other_request),
        Err(DecisionError::RequestMismatch("decision-99".into()))
    );

    let mut rebound = result_for(&request);
    rebound.prompt_sha256 = "0".repeat(64);
    assert_eq!(
        validate(&request, &rebound),
        Err(DecisionError::ResultBindingMismatch)
    );
}

#[test]
fn the_deterministic_baseline_is_deterministic_and_abstains_without_signal() {
    let baseline = DeterministicDecisionBaseline;

    let choice = request(DecisionKind::Choice);
    let first = baseline.decide(&choice);
    assert_eq!(first, baseline.decide(&choice));
    assert_eq!(first.choice.as_deref(), Some("click"));

    let unrelated = DecisionRequest {
        state: json!("Nothing here matches either option"),
        ..choice.clone()
    };
    let mut unrelated = unrelated;
    unrelated.bind();
    assert!(baseline.decide(&unrelated).abstain);

    let mut tied = request(DecisionKind::Choice);
    tied.options = vec![
        DecisionOption {
            id: "a".into(),
            description: "cart".into(),
        },
        DecisionOption {
            id: "b".into(),
            description: "cart".into(),
        },
    ];
    tied.bind();
    assert!(
        baseline.decide(&tied).abstain,
        "a tie must abstain rather than pick arbitrarily"
    );

    let noul = request(DecisionKind::Noul);
    let answer = baseline.decide(&noul);
    assert!(answer.noul.is_some_and(|p| (0.0..=1.0).contains(&p)));

    let score = request(DecisionKind::Score);
    let answer = baseline.decide(&score);
    assert!(answer.score.is_some_and(|p| (0.0..=1.0).contains(&p)));
}

#[test]
fn the_binding_is_a_sha256_over_the_criterion_and_options() {
    let request = request(DecisionKind::Choice);
    assert_eq!(request.prompt_sha256.len(), 64);
    assert!(
        request
            .prompt_sha256
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    assert!(request.prompt_preimage().contains("Which browser action"));
    assert!(request.prompt_preimage().contains("click"));
    // A stable digest, so a receipt from another machine can be checked.
    let mut known = DecisionRequest {
        id: "fixed".into(),
        kind: DecisionKind::Noul,
        state: json!("state"),
        criterion: "Is it so?".into(),
        options: vec![
            DecisionOption {
                id: "yes".into(),
                description: "It is".into(),
            },
            DecisionOption {
                id: "no".into(),
                description: "It is not".into(),
            },
        ],
        authority: ADVISORY_AUTHORITY.into(),
        threshold: None,
        calibration_ref: UNCALIBRATED.into(),
        prompt_sha256: String::new(),
    };
    known.bind();
    // A fixed digest, so a receipt produced on another machine can be checked
    // against the same criterion and option list.
    assert_eq!(
        known.prompt_sha256,
        "49ccd7aad924233a82ddc8f6410cadcc9062ef3788f44dd1735c1910d557d1cb"
    );
}
