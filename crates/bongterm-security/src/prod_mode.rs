//! Production safety mode. When on, dangerous commands escalate to deny.

use crate::EnforcementLevel;
use crate::dangerous::DangerKind;

/// Per-workspace toggle that hardens dangerous-command enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionSafetyMode {
    on: bool,
}

impl ProductionSafetyMode {
    #[must_use]
    pub fn on() -> Self {
        Self { on: true }
    }

    #[must_use]
    pub fn off() -> Self {
        Self { on: false }
    }

    #[must_use]
    pub fn is_on(self) -> bool {
        self.on
    }

    #[must_use]
    pub fn escalate(self, kind: DangerKind) -> EnforcementLevel {
        if self.on {
            EnforcementLevel::Deny
        } else {
            kind.enforcement()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dangerous::DangerKind;

    #[test]
    fn production_mode_escalates_dangerous_to_deny() {
        let mode = ProductionSafetyMode::on();
        assert_eq!(
            mode.escalate(DangerKind::GitForcePush),
            EnforcementLevel::Deny
        );
    }

    #[test]
    fn off_mode_preserves_require_approval() {
        let mode = ProductionSafetyMode::off();
        assert_eq!(
            mode.escalate(DangerKind::RecursiveDelete),
            EnforcementLevel::RequireApproval
        );
        assert!(!mode.is_on());
    }
}
