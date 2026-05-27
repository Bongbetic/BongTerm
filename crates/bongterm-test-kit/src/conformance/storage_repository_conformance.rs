//! Conformance suite for `BongTerm` storage repository port traits.
//!
//! Provides concrete conformance functions (not generic) that take boxed
//! repository trait objects. Also provides minimal in-memory mock
//! implementations used by the conformance tests below.

use bongterm_storage_api::{
    AgentRunId, BlockId, BlockRepo, CommandBlockRow, PaneId, PaneRepo, PaneRow, SessionId,
    SessionRepo, SessionRow, StorageError, WorkspaceId,
};
use std::collections::HashMap;
use std::sync::Mutex;
use time::OffsetDateTime;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Inline mock repos
// ---------------------------------------------------------------------------

/// In-memory mock for [`BlockRepo`].
pub struct MockBlockRepo {
    store: Mutex<HashMap<BlockId, CommandBlockRow>>,
}

impl MockBlockRepo {
    /// Create a new empty [`MockBlockRepo`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockBlockRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockRepo for MockBlockRepo {
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn insert_block(&self, row: &CommandBlockRow) -> Result<(), StorageError> {
        self.store.lock().unwrap().insert(row.id, row.clone());
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn get_block(&self, id: BlockId) -> Result<Option<CommandBlockRow>, StorageError> {
        Ok(self.store.lock().unwrap().get(&id).cloned())
    }
}

/// In-memory mock for [`PaneRepo`].
pub struct MockPaneRepo {
    store: Mutex<HashMap<PaneId, PaneRow>>,
}

impl MockPaneRepo {
    /// Create a new empty [`MockPaneRepo`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockPaneRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneRepo for MockPaneRepo {
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn insert_pane(&self, row: &PaneRow) -> Result<(), StorageError> {
        self.store.lock().unwrap().insert(row.id, row.clone());
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn get_pane(&self, id: PaneId) -> Result<Option<PaneRow>, StorageError> {
        Ok(self.store.lock().unwrap().get(&id).cloned())
    }
}

/// In-memory mock for [`SessionRepo`].
pub struct MockSessionRepo {
    store: Mutex<HashMap<SessionId, SessionRow>>,
}

impl MockSessionRepo {
    /// Create a new empty [`MockSessionRepo`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockSessionRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRepo for MockSessionRepo {
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn insert_session(&self, row: &SessionRow) -> Result<(), StorageError> {
        self.store.lock().unwrap().insert(row.id, row.clone());
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn get_session(&self, id: SessionId) -> Result<Option<SessionRow>, StorageError> {
        Ok(self.store.lock().unwrap().get(&id).cloned())
    }
}

// ---------------------------------------------------------------------------
// Conformance functions
// ---------------------------------------------------------------------------

/// Run happy-path conformance checks against any [`BlockRepo`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run_block_repo_conformance(repo: &dyn BlockRepo) {
    let block_id = BlockId(Uuid::nil());
    let pane_id = PaneId(Uuid::nil());
    let session_id = SessionId(Uuid::nil());
    let run_id = AgentRunId(Uuid::nil());
    let _ = run_id; // not used in block row

    let row = CommandBlockRow {
        id: block_id,
        pane_id,
        session_id,
        command: "cargo build".to_string(),
        exit_code: Some(0),
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
    };

    assert!(
        repo.insert_block(&row).is_ok(),
        "insert_block must return Ok"
    );

    let fetched = repo.get_block(block_id);
    assert!(fetched.is_ok(), "get_block must return Ok");
    assert!(
        fetched.unwrap().is_some(),
        "get_block must return Some after insert"
    );
}

/// Run happy-path conformance checks against any [`PaneRepo`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run_pane_repo_conformance(repo: &dyn PaneRepo) {
    let pane_id = PaneId(Uuid::nil());
    let session_id = SessionId(Uuid::nil());

    let row = PaneRow {
        id: pane_id,
        session_id,
        title: "conformance-pane".to_string(),
        cwd: "C:\\".to_string(),
    };

    assert!(repo.insert_pane(&row).is_ok(), "insert_pane must return Ok");

    let fetched = repo.get_pane(pane_id);
    assert!(fetched.is_ok(), "get_pane must return Ok");
    assert!(
        fetched.unwrap().is_some(),
        "get_pane must return Some after insert"
    );
}

/// Run happy-path conformance checks against any [`SessionRepo`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run_session_repo_conformance(repo: &dyn SessionRepo) {
    let session_id = SessionId(Uuid::nil());
    let workspace_id = WorkspaceId(Uuid::nil());

    let row = SessionRow {
        id: session_id,
        workspace_id,
        started_at: OffsetDateTime::UNIX_EPOCH,
    };

    assert!(
        repo.insert_session(&row).is_ok(),
        "insert_session must return Ok"
    );

    let fetched = repo.get_session(session_id);
    assert!(fetched.is_ok(), "get_session must return Ok");
    assert!(
        fetched.unwrap().is_some(),
        "get_session must return Some after insert"
    );
}
