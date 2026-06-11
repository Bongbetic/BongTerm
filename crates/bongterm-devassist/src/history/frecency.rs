//! Ctrl+R smart history: filter then rank by frecency (gate #11).
//!
//! Pure ranking over already-loaded entries. The DB read happens via
//! `bongterm_storage_api::FrecencyRepo` off the hot path; this function ranks
//! the in-memory candidate set so it is deterministically testable.

use bongterm_storage_api::{FrecencyRow, frecency_score};

use crate::history::filter::{HistoryEntryMeta, HistoryQuery};

/// Smart-history search engine.
pub struct SmartHistory;

impl SmartHistory {
    /// Filter `entries` by the parsed query, then rank survivors by frecency.
    /// Each tuple is `(metadata, use_count)`.
    #[must_use]
    pub fn search(
        raw_query: &str,
        entries: &[(HistoryEntryMeta, u64)],
        now_unix: i64,
    ) -> Vec<HistoryEntryMeta> {
        let query = HistoryQuery::parse(raw_query);
        let mut matched: Vec<(&HistoryEntryMeta, f64)> = entries
            .iter()
            .filter(|(meta, _)| query.matches(meta))
            .map(|(meta, count)| {
                let age_secs = i64::try_from(meta.age_secs).unwrap_or(i64::MAX);
                let row = FrecencyRow {
                    command: meta.command.clone(),
                    use_count: *count,
                    last_used_unix: now_unix.saturating_sub(age_secs),
                };
                (meta, frecency_score(&row, now_unix))
            })
            .collect();

        matched.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matched.into_iter().map(|(meta, _)| meta.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::filter::HistoryEntryMeta;

    fn entry(cmd: &str, shell: &str, exit: i64, age: u64, dur: f64) -> HistoryEntryMeta {
        HistoryEntryMeta {
            command: cmd.to_string(),
            cwd: "C:\\proj".to_string(),
            branch: Some("main".to_string()),
            agent: None,
            exit_code: Some(exit),
            shell: shell.to_string(),
            duration_secs: dur,
            age_secs: age,
        }
    }

    #[test]
    fn ctrl_r_search_filters_then_ranks_by_frecency() {
        let entries = vec![
            (entry("cargo build", "pwsh", 0, 3600, 30.0), 2_u64),
            (entry("cargo test", "pwsh", 0, 60, 12.0), 5_u64),
            (entry("git push", "cmd", 0, 30, 1.0), 9_u64),
        ];
        let results = SmartHistory::search("shell:pwsh cargo", &entries, 4000);
        assert_eq!(results.len(), 2, "git push filtered out by shell:pwsh");
        assert_eq!(results[0].command, "cargo test");
        assert_eq!(results[1].command, "cargo build");
    }

    #[test]
    fn ctrl_r_empty_query_returns_all_ranked() {
        let entries = vec![
            (entry("a", "pwsh", 0, 10, 1.0), 1_u64),
            (entry("b", "pwsh", 0, 10, 1.0), 10_u64),
        ];
        let results = SmartHistory::search("", &entries, 100);
        assert_eq!(results[0].command, "b");
    }
}
