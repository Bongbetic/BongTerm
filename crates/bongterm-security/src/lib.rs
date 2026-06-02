//! `BongTerm` security port traits.
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod dangerous;
pub mod forbidden;
pub mod prod_mode;
pub mod redactor;
pub mod trust;

use std::fmt;
use std::sync::{Arc, Mutex};

/// Enforcement strictness level — controls whether a policy decision is binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementLevel {
    /// Decision is informational only. User sees it; execution proceeds.
    Advisory,
    /// Decision requires explicit user approval before execution proceeds.
    RequireApproval,
    /// Decision is a hard block. Execution never proceeds.
    Deny,
}

impl fmt::Display for EnforcementLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Advisory => write!(f, "advisory"),
            Self::RequireApproval => write!(f, "require-approval"),
            Self::Deny => write!(f, "deny"),
        }
    }
}

/// The outcome of a policy evaluation.
#[derive(Debug, Clone)]
pub enum Decision {
    /// Action is permitted; proceed without user interaction.
    Allow,
    /// Action requires the user to explicitly approve before proceeding.
    RequireApproval {
        /// Human-readable reason for requiring approval.
        reason: String,
        /// Enforcement level for this decision.
        enforcement: EnforcementLevel,
    },
    /// Action is denied; never execute.
    Deny {
        /// Human-readable reason for the denial.
        reason: String,
        /// Enforcement level for this decision.
        enforcement: EnforcementLevel,
    },
    /// Informational warning. Execution proceeds; user is notified.
    Advisory {
        /// Warning message shown to the user.
        warn: String,
    },
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::RequireApproval { reason, .. } => write!(f, "require-approval: {reason}"),
            Self::Deny { reason, .. } => write!(f, "deny: {reason}"),
            Self::Advisory { warn } => write!(f, "advisory: {warn}"),
        }
    }
}

/// Describes the action being evaluated by the policy engine.
#[derive(Debug, Clone)]
pub struct PolicyRequest {
    /// Human-readable description of what is being requested.
    pub action: String,
    /// Risk classification for this request.
    pub risk: RiskClass,
    /// Optional workspace context.
    pub workspace_id: Option<String>,
}

/// Broad risk classification for policy routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskClass {
    /// Read-only filesystem or git operations.
    ReadOnly,
    /// Filesystem writes that are not destructive.
    WriteSafe,
    /// Destructive operations (rm, force-push, truncate).
    Destructive,
    /// Network operations.
    Network,
    /// Secret access.
    SecretAccess,
    /// Arbitrary shell execution.
    ArbitraryShell,
}

/// Port interface for the `BongTerm` policy engine.
///
/// Implementations live in `bongterm-security` (production) and test mocks.
pub trait PolicyEvaluator: Send + Sync {
    /// Evaluate a policy request and return the enforcement decision.
    fn evaluate(&self, request: &PolicyRequest) -> Decision;
}

/// A mock that can be configured to return specific decisions for tests.
pub struct MockPolicyEvaluator {
    responses: Arc<Mutex<Vec<Decision>>>,
    default: Decision,
}

impl MockPolicyEvaluator {
    /// Create a mock that always returns `Allow`.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            default: Decision::Allow,
        }
    }

    /// Create a mock that always returns `Deny`.
    #[must_use]
    pub fn deny_all() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            default: Decision::Deny {
                reason: "mock deny-all policy".to_string(),
                enforcement: EnforcementLevel::Deny,
            },
        }
    }

    /// Queue decisions to return in order (oldest first).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn queue(&self, decision: Decision) {
        self.responses.lock().unwrap().push(decision);
    }
}

impl PolicyEvaluator for MockPolicyEvaluator {
    fn evaluate(&self, _request: &PolicyRequest) -> Decision {
        let mut queue = self.responses.lock().unwrap();
        if queue.is_empty() {
            self.default.clone()
        } else {
            queue.remove(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advisory_display_never_contains_blocked() {
        let d = Decision::Advisory {
            warn: "use caution with force-push".to_string(),
        };
        let s = format!("{d}");
        assert!(
            !s.to_lowercase().contains("block"),
            "Advisory Display must not contain 'blocked', got: {s}"
        );
    }

    #[test]
    fn deny_display_contains_deny_not_block() {
        let d = Decision::Deny {
            reason: "destructive op".to_string(),
            enforcement: EnforcementLevel::Deny,
        };
        let s = format!("{d}");
        assert!(s.starts_with("deny:"), "expected 'deny:' prefix, got: {s}");
    }

    #[test]
    fn enforcement_level_display() {
        assert_eq!(format!("{}", EnforcementLevel::Advisory), "advisory");
        assert_eq!(
            format!("{}", EnforcementLevel::RequireApproval),
            "require-approval"
        );
        assert_eq!(format!("{}", EnforcementLevel::Deny), "deny");
    }

    #[test]
    fn permissive_mock_allows_all() {
        let mock = MockPolicyEvaluator::permissive();
        let req = PolicyRequest {
            action: "git push --force".to_string(),
            risk: RiskClass::Destructive,
            workspace_id: None,
        };
        assert!(matches!(mock.evaluate(&req), Decision::Allow));
    }

    #[test]
    fn deny_all_mock_denies() {
        let mock = MockPolicyEvaluator::deny_all();
        let req = PolicyRequest {
            action: "rm -rf /".to_string(),
            risk: RiskClass::Destructive,
            workspace_id: None,
        };
        assert!(matches!(mock.evaluate(&req), Decision::Deny { .. }));
    }

    #[test]
    fn queued_decision_returned_first() {
        let mock = MockPolicyEvaluator::permissive();
        mock.queue(Decision::Advisory {
            warn: "test warning".to_string(),
        });
        let req = PolicyRequest {
            action: "ls".to_string(),
            risk: RiskClass::ReadOnly,
            workspace_id: None,
        };
        assert!(matches!(mock.evaluate(&req), Decision::Advisory { .. }));
        // Second call uses default (Allow)
        assert!(matches!(mock.evaluate(&req), Decision::Allow));
    }
}
