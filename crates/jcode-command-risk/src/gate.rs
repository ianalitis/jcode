//! Stage 2: the authorization gate.
//!
//! Stage 1 ([`crate::assess`]) decides *whether* a command deserves scrutiny.
//! This decides *what happens next*, and it is deliberately not another model.
//!
//! # Why not a second model
//!
//! An LLM judge is expensive, adds latency to every borderline call, and can be
//! talked around by the same reasoning that produced the command. It also
//! creates a second thing to keep aligned.
//!
//! Instead the harness refuses and hands back a structured explanation. A
//! `Confirm` assessment can only proceed after the caller obtains independently
//! verified user approval. Model-authored prose never authorizes execution.
//!
//! This keeps authorization deterministic and adds no second model call.

use crate::{RiskAssessment, RiskLevel};

/// What the harness should do with a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// Run it.
    Allow,
    /// Refuse this attempt and require explicit user approval.
    Reflect { prompt: String },
    /// Refuse permanently. No justification unlocks this.
    Deny { reason: String },
}

/// Decide what to do with an assessed command.
pub fn gate(assessment: &RiskAssessment) -> GateOutcome {
    match assessment.level {
        RiskLevel::Safe | RiskLevel::Low => GateOutcome::Allow,
        RiskLevel::Catastrophic => GateOutcome::Deny {
            reason: format!(
                "This command is blocked and cannot be confirmed.\n\n{}\n\
                 If the user genuinely wants this, they must run it themselves \
                 outside the agent.",
                assessment.explanation()
            ),
        },
        RiskLevel::Confirm => GateOutcome::Reflect {
            prompt: reflection_prompt(assessment),
        },
    }
}

/// The text handed back to the model on a refused first attempt.
///
/// Phrasing is deliberate: it states the irreversible effect and makes clear
/// that model-authored prose cannot substitute for user authorization.
fn reflection_prompt(assessment: &RiskAssessment) -> String {
    format!(
        "This command was not run. It is irreversible:\n\n{}\n\
         This requires explicit user approval bound to the exact action. \
         Model-authored justification cannot authorize it.",
        assessment.explanation()
    )
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod gate_tests;
