//! Conformance suite for [`bongterm_security::PolicyEvaluator`].

use bongterm_security::{Decision, PolicyEvaluator, PolicyRequest, RiskClass};

/// Run happy-path conformance checks against any [`PolicyEvaluator`] implementation.
///
/// Pass a permissive evaluator. Asserts that a `ReadOnly` risk request does not
/// produce `Deny`.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(evaluator: &impl PolicyEvaluator) {
    let request = PolicyRequest {
        action: "read file".to_string(),
        risk: RiskClass::ReadOnly,
        workspace_id: None,
    };

    let decision = evaluator.evaluate(&request);
    assert!(
        !matches!(decision, Decision::Deny { .. }),
        "a permissive evaluator must not Deny a ReadOnly request"
    );
}
