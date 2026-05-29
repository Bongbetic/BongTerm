//! Approval queue. Routes a [`PolicyRequest`] through a [`PolicyEvaluator`]
//! and holds any non-allow decision for explicit user resolution, labeled
//! with its binding [`EnforcementLevel`]. A `Deny` can never be approved.

use bongterm_security::{Decision, EnforcementLevel, PolicyEvaluator, PolicyRequest};

/// Stable id for a queued approval item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApprovalId(pub u64);

/// State of a queued approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
}

/// User decision on a pending approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approve,
    Reject,
}

/// Whether the action may proceed immediately or is held pending a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Policy allowed the action; caller may proceed.
    Proceed,
    /// Action is held; the user must resolve the referenced approval.
    Held(ApprovalId),
}

/// One pending approval, with its human-readable reason and enforcement label.
#[derive(Debug, Clone)]
pub struct ApprovalItem {
    pub id: ApprovalId,
    pub action: String,
    pub reason: String,
    pub enforcement: EnforcementLevel,
    pub state: ApprovalState,
}

/// FIFO queue of held approvals.
#[derive(Debug, Default)]
pub struct ApprovalQueue {
    next_id: u64,
    items: Vec<ApprovalItem>,
}

impl ApprovalQueue {
    /// Evaluate `request`; allow -> `Proceed`, otherwise hold and return `Held`.
    pub fn submit(&mut self, evaluator: &dyn PolicyEvaluator, request: PolicyRequest) -> Gate {
        match evaluator.evaluate(&request) {
            Decision::Allow => Gate::Proceed,
            Decision::Advisory { warn } => {
                // Advisory does not block; record nothing, proceed.
                tracing::info!(action = %request.action, advisory = %warn, "advisory");
                Gate::Proceed
            }
            Decision::RequireApproval {
                reason,
                enforcement,
            } => self.hold(request.action, reason, enforcement),
            Decision::Deny {
                reason,
                enforcement,
            } => self.hold(request.action, reason, enforcement),
        }
    }

    fn hold(&mut self, action: String, reason: String, enforcement: EnforcementLevel) -> Gate {
        let id = ApprovalId(self.next_id);
        self.next_id += 1;
        self.items.push(ApprovalItem {
            id,
            action,
            reason,
            enforcement,
            state: ApprovalState::Pending,
        });
        Gate::Held(id)
    }

    /// Pending (unresolved) items only.
    #[must_use]
    pub fn pending(&self) -> Vec<ApprovalItem> {
        self.items
            .iter()
            .filter(|i| i.state == ApprovalState::Pending)
            .cloned()
            .collect()
    }

    /// Resolve a pending item. A `Deny`-enforced item is always `Rejected`,
    /// even if the user tries to approve it. Returns the resulting state, or
    /// `None` if the id is unknown / already resolved.
    pub fn resolve(&mut self, id: ApprovalId, decision: ApprovalDecision) -> Option<ApprovalState> {
        let item = self
            .items
            .iter_mut()
            .find(|i| i.id == id && i.state == ApprovalState::Pending)?;
        let new_state = match (item.enforcement, decision) {
            (EnforcementLevel::Deny, _) => ApprovalState::Rejected,
            (_, ApprovalDecision::Approve) => ApprovalState::Approved,
            (_, ApprovalDecision::Reject) => ApprovalState::Rejected,
        };
        item.state = new_state;
        Some(new_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_security::{Decision, EnforcementLevel, MockPolicyEvaluator, PolicyRequest, RiskClass};

    fn req(action: &str) -> PolicyRequest {
        PolicyRequest {
            action: action.to_string(),
            risk: RiskClass::Destructive,
            workspace_id: None,
        }
    }

    #[test]
    fn allow_decision_does_not_queue() {
        let eval = MockPolicyEvaluator::permissive();
        let mut q = ApprovalQueue::default();
        let gate = q.submit(&eval, req("ls"));
        assert_eq!(gate, Gate::Proceed);
        assert!(q.pending().is_empty());
    }

    #[test]
    fn require_approval_queues_with_enforcement_label() {
        let eval = MockPolicyEvaluator::permissive();
        eval.queue(Decision::RequireApproval {
            reason: "force push".to_string(),
            enforcement: EnforcementLevel::RequireApproval,
        });
        let mut q = ApprovalQueue::default();
        let gate = q.submit(&eval, req("git push --force"));
        assert!(matches!(gate, Gate::Held(_)));
        assert_eq!(q.pending().len(), 1);
        let item = &q.pending()[0];
        assert_eq!(item.enforcement, EnforcementLevel::RequireApproval);
        assert_eq!(item.state, ApprovalState::Pending);
        assert_eq!(item.reason, "force push");
    }

    #[test]
    fn deny_is_held_with_deny_enforcement_and_cannot_be_approved() {
        let eval = MockPolicyEvaluator::deny_all();
        let mut q = ApprovalQueue::default();
        let gate = q.submit(&eval, req("rm -rf /"));
        let id = match gate {
            Gate::Held(id) => id,
            Gate::Proceed => panic!("deny must be held"),
        };
        assert_eq!(q.pending()[0].enforcement, EnforcementLevel::Deny);
        // Attempting to approve a Deny is rejected.
        let resolved = q.resolve(id, ApprovalDecision::Approve);
        assert_eq!(resolved, Some(ApprovalState::Rejected));
    }

    #[test]
    fn user_approve_transitions_require_approval_to_approved() {
        let eval = MockPolicyEvaluator::permissive();
        eval.queue(Decision::RequireApproval {
            reason: "write file".to_string(),
            enforcement: EnforcementLevel::RequireApproval,
        });
        let mut q = ApprovalQueue::default();
        let id = match q.submit(&eval, req("write")) {
            Gate::Held(id) => id,
            Gate::Proceed => panic!(),
        };
        assert_eq!(
            q.resolve(id, ApprovalDecision::Approve),
            Some(ApprovalState::Approved)
        );
        assert!(
            q.pending().is_empty(),
            "resolved item leaves the pending queue"
        );
    }
}
