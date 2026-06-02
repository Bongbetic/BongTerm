//! Conformance checks for forbidden-abstraction process-tree auditors.

use bongterm_security::forbidden::{ForbiddenTechnique, ProcessObservation, ProcessTreeAuditor};

/// # Panics
///
/// Panics when the auditor misses a forbidden process-tree marker or flags clean input.
pub fn run_process_tree_auditor_conformance(auditor: &impl ProcessTreeAuditor) {
    let clean = auditor.scan(&[ProcessObservation {
        pid: 1,
        command_line: "bongterm.exe --conpty".to_string(),
    }]);
    assert!(clean.is_empty());

    let dirty = auditor.scan(&[ProcessObservation {
        pid: 2,
        command_line: "helper CreateRemoteThread".to_string(),
    }]);
    assert!(dirty.contains(&ForbiddenTechnique::DllInjection));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_security::forbidden::KeywordProcessTreeAuditor;

    #[test]
    fn keyword_auditor_satisfies_contract() {
        run_process_tree_auditor_conformance(&KeywordProcessTreeAuditor);
    }
}
