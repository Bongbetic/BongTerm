//! Workspace trust store. New folders default untrusted.

use std::collections::HashSet;
use std::sync::Mutex;

/// Trust state for workspace path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustState {
    Untrusted,
    Trusted,
}

/// Stores per-workspace trust decisions. Unknown = untrusted.
#[derive(Default)]
pub struct WorkspaceTrustStore {
    trusted: Mutex<HashSet<String>>,
}

impl WorkspaceTrustStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            trusted: Mutex::new(HashSet::new()),
        }
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn state(&self, path: &str) -> TrustState {
        if self.trusted.lock().unwrap().contains(path) {
            TrustState::Trusted
        } else {
            TrustState::Untrusted
        }
    }

    #[must_use]
    pub fn requires_prompt(&self, path: &str) -> bool {
        self.state(path) == TrustState::Untrusted
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn trust(&self, path: &str) {
        self.trusted.lock().unwrap().insert(path.to_string());
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn revoke(&self, path: &str) {
        self.trusted.lock().unwrap().remove(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_opened_workspace_defaults_untrusted() {
        let store = WorkspaceTrustStore::new();
        assert_eq!(store.state("C:/repos/unknown"), TrustState::Untrusted);
        assert!(store.requires_prompt("C:/repos/unknown"));
    }

    #[test]
    fn explicitly_trusting_persists_decision() {
        let store = WorkspaceTrustStore::new();
        store.trust("C:/repos/myproj");
        assert_eq!(store.state("C:/repos/myproj"), TrustState::Trusted);
        assert!(!store.requires_prompt("C:/repos/myproj"));
    }

    #[test]
    fn revoking_returns_to_untrusted() {
        let store = WorkspaceTrustStore::new();
        store.trust("C:/repos/myproj");
        store.revoke("C:/repos/myproj");
        assert_eq!(store.state("C:/repos/myproj"), TrustState::Untrusted);
    }
}
