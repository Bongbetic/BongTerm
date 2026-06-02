//! Crash recovery screen model.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashClass {
    PanePanic,
    RendererPanic,
    McpCrashLoop,
    SqliteBusy,
    SidecarTornWrite,
    DiskQuota,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    Restore,
    Discard,
    ExportDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryScreen {
    pub crash: CrashClass,
    pub actions: Vec<RecoveryAction>,
}

impl RecoveryScreen {
    #[must_use]
    pub fn for_crash(crash: CrashClass) -> Self {
        Self {
            crash,
            actions: vec![
                RecoveryAction::Restore,
                RecoveryAction::Discard,
                RecoveryAction::ExportDiagnostics,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_screen_always_offers_restore_discard_export() {
        let screen = RecoveryScreen::for_crash(CrashClass::RendererPanic);
        assert_eq!(
            screen.actions,
            vec![
                RecoveryAction::Restore,
                RecoveryAction::Discard,
                RecoveryAction::ExportDiagnostics
            ]
        );
    }
}
