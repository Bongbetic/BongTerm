//! Conformance for `bongterm_storage_api::FrecencyRepo` plus an in-memory mock.

use std::collections::HashMap;
use std::sync::Mutex;

use bongterm_storage_api::{FrecencyRepo, FrecencyRow, StorageError};

/// In-memory mock for [`FrecencyRepo`].
pub struct MockFrecencyRepo {
    store: Mutex<HashMap<String, FrecencyRow>>,
}

impl MockFrecencyRepo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockFrecencyRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl FrecencyRepo for MockFrecencyRepo {
    fn record_use(&self, command: &str, at_unix: i64) -> Result<(), StorageError> {
        let mut guard = self.store.lock().expect("frecency mock mutex");
        let row = guard.entry(command.to_string()).or_insert(FrecencyRow {
            command: command.to_string(),
            use_count: 0,
            last_used_unix: at_unix,
        });
        row.use_count += 1;
        row.last_used_unix = at_unix;
        Ok(())
    }

    fn top_n(&self, n: usize, now_unix: i64) -> Result<Vec<FrecencyRow>, StorageError> {
        let guard = self.store.lock().expect("frecency mock mutex");
        let mut rows: Vec<FrecencyRow> = guard.values().cloned().collect();
        rows.sort_by(|a, b| {
            let score_a = bongterm_storage_api::frecency_score(a, now_unix);
            let score_b = bongterm_storage_api::frecency_score(b, now_unix);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        rows.truncate(n);
        Ok(rows)
    }
}

/// Run happy-path conformance against any [`FrecencyRepo`].
///
/// # Panics
/// Panics on contract violation.
pub fn run_frecency_repo_conformance(repo: &dyn FrecencyRepo) {
    repo.record_use("cargo build", 1000).unwrap();
    repo.record_use("cargo build", 2000).unwrap();
    repo.record_use("git status", 1500).unwrap();
    let top = repo.top_n(10, 3000).unwrap();
    assert!(!top.is_empty(), "top_n must return recorded entries");
    assert_eq!(top[0].command, "cargo build");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_satisfies_conformance() {
        run_frecency_repo_conformance(&MockFrecencyRepo::new());
    }
}
