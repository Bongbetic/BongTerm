//! Transcript persistence sink. Drives a [`TranscriptRepo`] with monotonic
//! chunk indices. On persistence failure it transitions to `paused` rather
//! than blocking the live output stream.

use bongterm_storage_api::{AgentRunId, TranscriptId, TranscriptRepo, TranscriptRow};
use uuid::Uuid;

/// Append-only transcript writer for one agent run.
pub struct TranscriptSink {
    run_id: AgentRunId,
    next_index: u64,
    paused: bool,
}

impl TranscriptSink {
    /// Create a sink for one run, starting at chunk index 0.
    #[must_use]
    pub fn new(run_id: AgentRunId) -> Self {
        Self {
            run_id,
            next_index: 0,
            paused: false,
        }
    }

    /// Whether persistence is paused due to a previous storage error.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Persist one transcript chunk without blocking caller progress.
    pub fn write(&mut self, repo: &dyn TranscriptRepo, text: &str) {
        let row = TranscriptRow {
            id: TranscriptId(Uuid::new_v4()),
            agent_run_id: self.run_id,
            chunk_index: self.next_index,
            text: text.to_string(),
        };

        match repo.append_chunk(&row) {
            Ok(()) => {
                self.next_index += 1;
                self.paused = false;
            }
            Err(error) => {
                tracing::warn!(run_id = ?self.run_id, %error, "transcript persistence paused");
                self.paused = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_storage_api::{AgentRunId, StorageError, TranscriptRepo, TranscriptRow};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    struct VecRepo {
        rows: Mutex<Vec<TranscriptRow>>,
    }

    impl TranscriptRepo for VecRepo {
        fn append_chunk(&self, row: &TranscriptRow) -> Result<(), StorageError> {
            self.rows
                .lock()
                .expect("rows mutex poisoned")
                .push(row.clone());
            Ok(())
        }

        fn list_chunks(&self, run_id: AgentRunId) -> Result<Vec<TranscriptRow>, StorageError> {
            Ok(self
                .rows
                .lock()
                .expect("rows mutex poisoned")
                .iter()
                .filter(|row| row.agent_run_id == run_id)
                .cloned()
                .collect())
        }
    }

    #[test]
    fn sink_appends_monotonic_chunks() {
        let repo = VecRepo::default();
        let run = AgentRunId(Uuid::nil());
        let mut sink = TranscriptSink::new(run);
        sink.write(&repo, "first line");
        sink.write(&repo, "second line");
        let chunks = repo.list_chunks(run).expect("list chunks");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].chunk_index, 1);
        assert_eq!(chunks[0].text, "first line");
    }

    #[test]
    fn sink_records_paused_on_repo_error_without_panicking() {
        struct FailRepo;

        impl TranscriptRepo for FailRepo {
            fn append_chunk(&self, _row: &TranscriptRow) -> Result<(), StorageError> {
                Err(StorageError::Database("disk full".to_string()))
            }

            fn list_chunks(&self, _run_id: AgentRunId) -> Result<Vec<TranscriptRow>, StorageError> {
                Ok(vec![])
            }
        }

        let mut sink = TranscriptSink::new(AgentRunId(Uuid::nil()));
        assert!(!sink.is_paused());
        sink.write(&FailRepo, "x");
        assert!(
            sink.is_paused(),
            "sink must mark paused on persistence failure"
        );
    }
}
