//! `BongTerm` storage port traits and DTOs.
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.
//!
//! Concrete implementations live in `bongterm-storage-sqlite` and are wired
//! only by `bongterm-app`.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

// ---------------------------------------------------------------------------
// Newtype IDs
// ---------------------------------------------------------------------------

/// Unique identifier for a command block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BlockId(pub uuid::Uuid);

/// Unique identifier for a terminal pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PaneId(pub uuid::Uuid);

/// Unique identifier for a terminal session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionId(pub uuid::Uuid);

/// Unique identifier for a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceId(pub uuid::Uuid);

/// Unique identifier for a transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TranscriptId(pub uuid::Uuid);

/// Unique identifier for an agent run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AgentRunId(pub uuid::Uuid);

/// Unique identifier for an MCP tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct McpCallId(pub uuid::Uuid);

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// A single command block recorded from a terminal pane.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandBlockRow {
    pub id: BlockId,
    pub pane_id: PaneId,
    pub session_id: SessionId,
    pub command: String,
    pub exit_code: Option<i64>,
    pub started_at: time::OffsetDateTime,
    pub finished_at: Option<time::OffsetDateTime>,
}

/// A terminal pane within a session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaneRow {
    pub id: PaneId,
    pub session_id: SessionId,
    pub title: String,
    pub cwd: String,
}

/// A terminal session within a workspace.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionRow {
    pub id: SessionId,
    pub workspace_id: WorkspaceId,
    pub started_at: time::OffsetDateTime,
}

/// A `BongTerm` workspace (project root).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceRow {
    pub id: WorkspaceId,
    pub name: String,
    pub root_path: String,
}

/// A single append-only transcript chunk from an agent run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptRow {
    pub id: TranscriptId,
    pub agent_run_id: AgentRunId,
    pub chunk_index: u64,
    pub text: String,
}

/// A single agent execution run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentRunRow {
    pub id: AgentRunId,
    pub session_id: SessionId,
    pub adapter_name: String,
    pub started_at: time::OffsetDateTime,
    pub exit_code: Option<i64>,
}

/// A single MCP tool call made during an agent run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpCallRow {
    pub id: McpCallId,
    pub agent_run_id: AgentRunId,
    pub tool_name: String,
    pub duration_ms: u64,
    pub succeeded: bool,
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors returned by storage port implementations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

// ---------------------------------------------------------------------------
// Repository traits
// ---------------------------------------------------------------------------

/// Persist and retrieve [`CommandBlockRow`] records.
pub trait BlockRepo: Send + Sync + 'static {
    fn insert_block(&self, row: &CommandBlockRow) -> Result<(), StorageError>;
    fn get_block(&self, id: BlockId) -> Result<Option<CommandBlockRow>, StorageError>;
}

/// Persist and retrieve [`PaneRow`] records.
pub trait PaneRepo: Send + Sync + 'static {
    fn insert_pane(&self, row: &PaneRow) -> Result<(), StorageError>;
    fn get_pane(&self, id: PaneId) -> Result<Option<PaneRow>, StorageError>;
}

/// Persist and retrieve [`SessionRow`] records.
pub trait SessionRepo: Send + Sync + 'static {
    fn insert_session(&self, row: &SessionRow) -> Result<(), StorageError>;
    fn get_session(&self, id: SessionId) -> Result<Option<SessionRow>, StorageError>;
}

/// Persist and retrieve [`WorkspaceRow`] records.
pub trait WorkspaceRepo: Send + Sync + 'static {
    fn insert_workspace(&self, row: &WorkspaceRow) -> Result<(), StorageError>;
    fn get_workspace(&self, id: WorkspaceId) -> Result<Option<WorkspaceRow>, StorageError>;
}

/// Append and list [`TranscriptRow`] chunks for an agent run.
pub trait TranscriptRepo: Send + Sync + 'static {
    fn append_chunk(&self, row: &TranscriptRow) -> Result<(), StorageError>;
    fn list_chunks(&self, run_id: AgentRunId) -> Result<Vec<TranscriptRow>, StorageError>;
}

/// Persist and retrieve [`AgentRunRow`] records.
pub trait AgentRunRepo: Send + Sync + 'static {
    fn insert_run(&self, row: &AgentRunRow) -> Result<(), StorageError>;
    fn get_run(&self, id: AgentRunId) -> Result<Option<AgentRunRow>, StorageError>;
}

/// Persist and list [`McpCallRow`] records.
pub trait McpCallRepo: Send + Sync + 'static {
    fn insert_call(&self, row: &McpCallRow) -> Result<(), StorageError>;
    fn list_calls(&self, run_id: AgentRunId) -> Result<Vec<McpCallRow>, StorageError>;
}

/// Record MCP tool-use approvals for audit.
pub trait ToolAuditRepo: Send + Sync + 'static {
    fn record_tool_use(
        &self,
        run_id: AgentRunId,
        tool: &str,
        approved: bool,
    ) -> Result<(), StorageError>;
}

/// Record secret-access events for audit.
pub trait SecretAuditRepo: Send + Sync + 'static {
    fn record_secret_access(
        &self,
        run_id: AgentRunId,
        secret_name: &str,
        consumer: &str,
    ) -> Result<(), StorageError>;
}

/// Append resource-ledger samples (RSS, CPU).
pub trait LedgerRepo: Send + Sync + 'static {
    fn record_sample(
        &self,
        ts: time::OffsetDateTime,
        rss_bytes: u64,
        cpu_percent: f32,
    ) -> Result<(), StorageError>;
}

/// Run pending schema migrations. Returns count of migrations applied.
pub trait MigrationRunner: Send + Sync + 'static {
    fn run_migrations(&self) -> Result<u32, StorageError>;
}

// ---------------------------------------------------------------------------
// Frecency (smart-history ranking)
// ---------------------------------------------------------------------------

/// A frecency record for one command string.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrecencyRow {
    pub command: String,
    pub use_count: u64,
    pub last_used_unix: i64,
}

/// Combined recency + frequency score. Higher is more relevant.
///
/// Frequency contributes logarithmically; recency decays with elapsed time.
/// Pure function so the `SQLite` impl and the mock rank identically.
#[allow(clippy::doc_markdown, clippy::cast_precision_loss)]
#[must_use]
pub fn frecency_score(row: &FrecencyRow, now_unix: i64) -> f64 {
    let frequency = (1.0 + row.use_count as f64).ln();
    let age_secs = (now_unix - row.last_used_unix).max(0) as f64;
    let recency = 0.5_f64.powf(age_secs / 86_400.0);
    frequency * (0.5 + recency)
}

/// Record command uses and retrieve frecency-ranked history.
pub trait FrecencyRepo: Send + Sync + 'static {
    /// Record one use of `command` at `at_unix` (seconds since epoch).
    fn record_use(&self, command: &str, at_unix: i64) -> Result<(), StorageError>;

    /// Return the top `n` commands by frecency score as of `now_unix`.
    fn top_n(&self, n: usize, now_unix: i64) -> Result<Vec<FrecencyRow>, StorageError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn nil_uuid() -> uuid::Uuid {
        uuid::Uuid::nil()
    }

    #[test]
    fn command_block_row_serde_roundtrip() {
        let row = CommandBlockRow {
            id: BlockId(nil_uuid()),
            pane_id: PaneId(nil_uuid()),
            session_id: SessionId(nil_uuid()),
            command: "cargo build".to_string(),
            exit_code: Some(0),
            started_at: time::OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(time::OffsetDateTime::UNIX_EPOCH),
        };
        let json = serde_json::to_string(&row).expect("serialize");
        let back: CommandBlockRow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.command, row.command);
        assert_eq!(back.exit_code, row.exit_code);
        assert_eq!(back.id.0, row.id.0);
    }

    #[test]
    fn pane_row_serde_roundtrip() {
        let row = PaneRow {
            id: PaneId(nil_uuid()),
            session_id: SessionId(nil_uuid()),
            title: "main".to_string(),
            cwd: "C:\\".to_string(),
        };
        let json = serde_json::to_string(&row).expect("serialize");
        let back: PaneRow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.title, row.title);
        assert_eq!(back.cwd, row.cwd);
    }

    #[test]
    fn session_row_serde_roundtrip() {
        let row = SessionRow {
            id: SessionId(nil_uuid()),
            workspace_id: WorkspaceId(nil_uuid()),
            started_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        let json = serde_json::to_string(&row).expect("serialize");
        let back: SessionRow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id.0, row.id.0);
        assert_eq!(back.workspace_id.0, row.workspace_id.0);
    }

    #[test]
    fn workspace_row_serde_roundtrip() {
        let row = WorkspaceRow {
            id: WorkspaceId(nil_uuid()),
            name: "my-project".to_string(),
            root_path: "C:\\Projects\\foo".to_string(),
        };
        let json = serde_json::to_string(&row).expect("serialize");
        let back: WorkspaceRow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, row.name);
        assert_eq!(back.root_path, row.root_path);
    }

    #[test]
    fn transcript_row_serde_roundtrip() {
        let row = TranscriptRow {
            id: TranscriptId(nil_uuid()),
            agent_run_id: AgentRunId(nil_uuid()),
            chunk_index: 7,
            text: "hello agent".to_string(),
        };
        let json = serde_json::to_string(&row).expect("serialize");
        let back: TranscriptRow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.chunk_index, row.chunk_index);
        assert_eq!(back.text, row.text);
    }

    #[test]
    fn agent_run_row_serde_roundtrip() {
        let row = AgentRunRow {
            id: AgentRunId(nil_uuid()),
            session_id: SessionId(nil_uuid()),
            adapter_name: "claude-code".to_string(),
            started_at: time::OffsetDateTime::UNIX_EPOCH,
            exit_code: None,
        };
        let json = serde_json::to_string(&row).expect("serialize");
        let back: AgentRunRow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.adapter_name, row.adapter_name);
        assert_eq!(back.exit_code, row.exit_code);
    }

    #[test]
    fn mcp_call_row_serde_roundtrip() {
        let row = McpCallRow {
            id: McpCallId(nil_uuid()),
            agent_run_id: AgentRunId(nil_uuid()),
            tool_name: "read_file".to_string(),
            duration_ms: 42,
            succeeded: true,
        };
        let json = serde_json::to_string(&row).expect("serialize");
        let back: McpCallRow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.tool_name, row.tool_name);
        assert_eq!(back.duration_ms, row.duration_ms);
        assert_eq!(back.succeeded, row.succeeded);
    }
}
