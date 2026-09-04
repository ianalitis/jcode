//! Gate tests.
//!
//! The central property: a blind retry must not satisfy the gate. That is what
//! separates this from a "run it twice" speed bump an agent defeats reflexively.

use super::*;
use crate::{RiskContext, assess};
use std::path::PathBuf;

fn ctx() -> RiskContext {
    RiskContext {
        working_dir: Some(PathBuf::from("/home/u/proj")),
        home_dir: Some(PathBuf::from("/home/u")),
    }
}

#[test]
fn safe_commands_pass_straight_through() {
    let assessment = assess("ls -la", &ctx());
    assert_eq!(gate(&assessment), GateOutcome::Allow);
}

#[test]
fn in_project_cleanup_is_not_interrupted() {
    let assessment = assess("rm -rf target", &ctx());
    assert_eq!(gate(&assessment), GateOutcome::Allow);
}

#[test]
fn catastrophic_is_denied_unconditionally() {
    let assessment = assess("rm -rf ~", &ctx());
    match gate(&assessment) {
        GateOutcome::Deny { reason } => {
            assert!(reason.contains("cannot be confirmed"), "{reason}");
        }
        other => panic!("catastrophic must be unconditional, got {other:?}"),
    }
}

#[test]
fn first_attempt_at_a_risky_command_is_refused_with_a_reflection_prompt() {
    let assessment = assess("rm -rf $TARGET", &ctx());
    match gate(&assessment) {
        GateOutcome::Reflect { prompt } => {
            // The prompt must point at the user's request, not at generic caution.
            assert!(prompt.contains("explicit user approval"), "{prompt}");
            assert!(prompt.contains("cannot authorize"), "{prompt}");
        }
        other => panic!("expected a reflection prompt, got {other:?}"),
    }
}

#[test]
fn a_blind_retry_does_not_satisfy_the_gate() {
    // This is the property that makes it more than a speed bump: repeating the
    // identical call, with no added reasoning, fails exactly as before.
    let assessment = assess("rm -rf $TARGET", &ctx());
    let first = gate(&assessment);
    let second = gate(&assessment);
    assert_eq!(first, second);
    assert!(matches!(second, GateOutcome::Reflect { .. }));
}

#[test]
fn model_prose_cannot_satisfy_the_gate() {
    let assessment = assess("rm -rf $TARGET", &ctx());
    assert!(matches!(gate(&assessment), GateOutcome::Reflect { .. }));
}

#[test]
fn reflection_prompt_names_the_specific_path() {
    let assessment = assess("rm -rf $TARGET", &ctx());
    let GateOutcome::Reflect { prompt } = gate(&assessment) else {
        panic!("expected reflection");
    };
    assert!(
        prompt.contains("$TARGET"),
        "the model needs to see what it was about to destroy: {prompt}"
    );
}
