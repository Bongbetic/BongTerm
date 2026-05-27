//! Negative conformance suite — verifies safety invariants hold against mocks.
//!
//! Invariants verified:
//! 1. Missing secret resolves to error (fails closed, not open).
//! 2. Advisory decision display never contains "blocked".
//! 3. Deny decision never returns Ok from mock spawn (just asserts `Decision::Deny`).
//! 4. `RequireApproval` decision does not self-approve.
//! 5. Redacted exports never contain known synthetic token.

#[cfg(test)]
mod tests {
    use bongterm_secrets_api::{
        ConsumerId, ResolveError, SecretRef, SecretScope, SecretStore, SecretValue,
    };
    use bongterm_security::{
        Decision, EnforcementLevel, MockPolicyEvaluator, PolicyEvaluator, PolicyRequest, RiskClass,
    };

    // -------------------------------------------------------------------------
    // Inline mock SecretStore that always returns Missing
    // -------------------------------------------------------------------------

    struct AlwaysMissingStore;

    impl SecretStore for AlwaysMissingStore {
        fn resolve(
            &self,
            secret: &SecretRef,
            _consumer: &ConsumerId,
        ) -> Result<SecretValue, ResolveError> {
            Err(ResolveError::Missing(secret.clone()))
        }

        fn exists(&self, _secret: &SecretRef) -> bool {
            false
        }
    }

    // -------------------------------------------------------------------------
    // Test 1 — missing secret resolves to error
    // -------------------------------------------------------------------------

    #[test]
    fn missing_secret_resolves_to_error() {
        let store = AlwaysMissingStore;
        let secret_ref = SecretRef {
            name: "NONEXISTENT_SECRET".to_string(),
            scope: SecretScope::Global,
        };
        let consumer = ConsumerId("test-consumer".to_string());
        let result = store.resolve(&secret_ref, &consumer);
        assert!(
            matches!(result, Err(ResolveError::Missing(_))),
            "missing secret must resolve to Err(ResolveError::Missing(_)), got: {result:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Test 2 — Advisory display never contains "blocked"
    // -------------------------------------------------------------------------

    #[test]
    fn advisory_display_never_contains_blocked() {
        let decision = Decision::Advisory {
            warn: "be careful with force operations".to_string(),
        };
        let s = format!("{decision}");
        assert!(
            !s.to_lowercase().contains("block"),
            "Advisory Display must not contain 'block', got: {s}"
        );
    }

    // -------------------------------------------------------------------------
    // Test 3 — Deny decision variant is Deny
    // -------------------------------------------------------------------------

    #[test]
    fn deny_decision_variant_is_deny() {
        let evaluator = MockPolicyEvaluator::deny_all();
        let request = PolicyRequest {
            action: "rm -rf /".to_string(),
            risk: RiskClass::Destructive,
            workspace_id: None,
        };
        let decision = evaluator.evaluate(&request);
        assert!(
            matches!(decision, Decision::Deny { .. }),
            "deny_all() evaluator must return Decision::Deny for Destructive request, got: {decision:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Test 4 — RequireApproval does not self-approve
    // -------------------------------------------------------------------------

    #[test]
    fn require_approval_does_not_self_approve() {
        let evaluator = MockPolicyEvaluator::permissive();
        evaluator.queue(Decision::RequireApproval {
            reason: "test".to_string(),
            enforcement: EnforcementLevel::RequireApproval,
        });
        let request = PolicyRequest {
            action: "deploy".to_string(),
            risk: RiskClass::Destructive,
            workspace_id: None,
        };
        let decision = evaluator.evaluate(&request);
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "queued RequireApproval must be returned as RequireApproval, not self-approved"
        );
        assert!(
            !matches!(decision, Decision::Allow),
            "RequireApproval must NOT be silently converted to Allow"
        );
    }

    // -------------------------------------------------------------------------
    // Test 5 — SecretValue Display and Debug do not leak plaintext
    // -------------------------------------------------------------------------

    #[test]
    fn secret_value_display_does_not_leak_plaintext() {
        let token = "SYNTHETIC_TOKEN_abc123xyz";
        let value = SecretValue::from_plaintext(token.to_string());

        let display = format!("{value}");
        let debug = format!("{value:?}");

        assert!(
            !display.contains(token),
            "SecretValue Display must not leak plaintext, got: {display}"
        );
        assert!(
            !debug.contains(token),
            "SecretValue Debug must not leak plaintext, got: {debug}"
        );
    }
}
