//! SQLite-backed implementation of `bongterm-storage-api` port traits.
//!
//! # Safety
//!
//! `rusqlite::Connection` is `!Send` because it holds thread-local `SQLite`
//! error state. We wrap it in a `parking_lot::Mutex` which guarantees
//! exclusive access at any given time. The `unsafe impl Send + Sync` below is
//! sound: we never rely on the thread-local error state — all errors propagate
//! through `Result`.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

pub mod sidecar;

use bongterm_storage_api::{
    AgentRunId, AgentRunRepo, AgentRunRow, BlockId, BlockRepo, CommandBlockRow, LedgerRepo,
    McpCallId, McpCallRepo, McpCallRow, MigrationRunner, PaneId, PaneRepo, PaneRow, SessionId,
    SessionRepo, SessionRow, StorageError, TranscriptId, TranscriptRepo, TranscriptRow,
    WorkspaceId, WorkspaceRepo, WorkspaceRow,
};
use parking_lot::Mutex;

// ---------------------------------------------------------------------------
// Migration SQL
// ---------------------------------------------------------------------------

const MIGRATION_0001: &str = "
CREATE TABLE IF NOT EXISTS workspaces (
    id        TEXT NOT NULL PRIMARY KEY,
    name      TEXT NOT NULL,
    root_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id           TEXT NOT NULL PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    started_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS panes (
    id         TEXT NOT NULL PRIMARY KEY,
    session_id TEXT NOT NULL,
    title      TEXT NOT NULL,
    cwd        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS command_blocks (
    id          TEXT    NOT NULL PRIMARY KEY,
    pane_id     TEXT    NOT NULL,
    session_id  TEXT    NOT NULL,
    command     TEXT    NOT NULL,
    exit_code   INTEGER,
    started_at  TEXT    NOT NULL,
    finished_at TEXT
);

CREATE TABLE IF NOT EXISTS agent_runs (
    id           TEXT    NOT NULL PRIMARY KEY,
    session_id   TEXT    NOT NULL,
    adapter_name TEXT    NOT NULL,
    started_at   TEXT    NOT NULL,
    exit_code    INTEGER
);

CREATE TABLE IF NOT EXISTS transcripts (
    id           TEXT    NOT NULL PRIMARY KEY,
    agent_run_id TEXT    NOT NULL,
    chunk_index  INTEGER NOT NULL,
    text         TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS mcp_calls (
    id           TEXT    NOT NULL PRIMARY KEY,
    agent_run_id TEXT    NOT NULL,
    tool_name    TEXT    NOT NULL,
    duration_ms  INTEGER NOT NULL,
    succeeded    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS ledger_samples (
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    ts          TEXT    NOT NULL,
    rss_bytes   INTEGER NOT NULL,
    cpu_percent REAL    NOT NULL
);
";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Takes the error by value so it can be used directly as `.map_err(db_err)` at
// ~20 call sites; `map_or`/`map_err` hand the closure an owned `rusqlite::Error`.
#[allow(clippy::needless_pass_by_value)]
fn db_err(e: rusqlite::Error) -> StorageError {
    StorageError::Database(e.to_string())
}

fn encode_uuid(id: uuid::Uuid) -> String {
    id.to_string()
}

fn decode_uuid(s: &str) -> Result<uuid::Uuid, StorageError> {
    uuid::Uuid::parse_str(s).map_err(|e| StorageError::Database(format!("uuid: {e}")))
}

fn encode_dt(dt: time::OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .expect("OffsetDateTime must serialize as RFC 3339")
}

fn decode_dt(s: &str) -> Result<time::OffsetDateTime, StorageError> {
    time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .map_err(|e| StorageError::Database(format!("datetime: {e}")))
}

// ---------------------------------------------------------------------------
// SqliteStore
// ---------------------------------------------------------------------------

/// SQLite-backed implementation of all `bongterm-storage-api` repository traits.
///
/// Call [`run_migrations`][MigrationRunner::run_migrations] once after opening
/// to create the schema before using any repository methods.
pub struct SqliteStore {
    conn: Mutex<rusqlite::Connection>,
}

#[allow(unsafe_code)]
// SAFETY: rusqlite::Connection is !Send solely because SQLite's thread-local
// error-reporting state. The Mutex enforces exclusive access and we never
// inspect the thread-local error state — all errors are returned via Result.
unsafe impl Send for SqliteStore {}

#[allow(unsafe_code)]
// SAFETY: See Send impl above.
unsafe impl Sync for SqliteStore {}

impl SqliteStore {
    /// Open or create a `SqliteStore` at `path` with WAL journal mode.
    pub fn open(path: &std::path::Path) -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open(path).map_err(db_err)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .map_err(db_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open_in_memory().map_err(db_err)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(db_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

// ---------------------------------------------------------------------------
// MigrationRunner
// ---------------------------------------------------------------------------

impl MigrationRunner for SqliteStore {
    fn run_migrations(&self) -> Result<u32, StorageError> {
        let conn = self.conn.lock();

        // Always ensure the migrations tracking table exists.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                name       TEXT    NOT NULL UNIQUE,
                applied_at TEXT    NOT NULL
            );",
        )
        .map_err(db_err)?;

        // Check whether migration 0001_init was already applied.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations WHERE name = '0001_init'",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;

        if count > 0 {
            return Ok(0);
        }

        conn.execute_batch(MIGRATION_0001).map_err(db_err)?;
        conn.execute(
            "INSERT INTO _migrations (name, applied_at) VALUES (?1, ?2)",
            rusqlite::params!["0001_init", encode_dt(time::OffsetDateTime::now_utc())],
        )
        .map_err(db_err)?;

        Ok(1)
    }
}

// ---------------------------------------------------------------------------
// WorkspaceRepo
// ---------------------------------------------------------------------------

impl WorkspaceRepo for SqliteStore {
    fn insert_workspace(&self, row: &WorkspaceRow) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, root_path) VALUES (?1, ?2, ?3)",
            rusqlite::params![encode_uuid(row.id.0), &row.name, &row.root_path],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn get_workspace(&self, id: WorkspaceId) -> Result<Option<WorkspaceRow>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id, name, root_path FROM workspaces WHERE id = ?1",
            rusqlite::params![encode_uuid(id.0)],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        );
        match result {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
            Ok((id_s, name, root_path)) => Ok(Some(WorkspaceRow {
                id: WorkspaceId(decode_uuid(&id_s)?),
                name,
                root_path,
            })),
        }
    }
}

// ---------------------------------------------------------------------------
// SessionRepo
// ---------------------------------------------------------------------------

impl SessionRepo for SqliteStore {
    fn insert_session(&self, row: &SessionRow) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO sessions (id, workspace_id, started_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![
                encode_uuid(row.id.0),
                encode_uuid(row.workspace_id.0),
                encode_dt(row.started_at),
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn get_session(&self, id: SessionId) -> Result<Option<SessionRow>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id, workspace_id, started_at FROM sessions WHERE id = ?1",
            rusqlite::params![encode_uuid(id.0)],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        );
        match result {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
            Ok((id_s, ws_s, started_s)) => Ok(Some(SessionRow {
                id: SessionId(decode_uuid(&id_s)?),
                workspace_id: WorkspaceId(decode_uuid(&ws_s)?),
                started_at: decode_dt(&started_s)?,
            })),
        }
    }
}

// ---------------------------------------------------------------------------
// PaneRepo
// ---------------------------------------------------------------------------

impl PaneRepo for SqliteStore {
    fn insert_pane(&self, row: &PaneRow) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO panes (id, session_id, title, cwd)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                encode_uuid(row.id.0),
                encode_uuid(row.session_id.0),
                &row.title,
                &row.cwd,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn get_pane(&self, id: PaneId) -> Result<Option<PaneRow>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id, session_id, title, cwd FROM panes WHERE id = ?1",
            rusqlite::params![encode_uuid(id.0)],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        );
        match result {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
            Ok((id_s, sess_s, title, cwd)) => Ok(Some(PaneRow {
                id: PaneId(decode_uuid(&id_s)?),
                session_id: SessionId(decode_uuid(&sess_s)?),
                title,
                cwd,
            })),
        }
    }
}

// ---------------------------------------------------------------------------
// BlockRepo
// ---------------------------------------------------------------------------

impl BlockRepo for SqliteStore {
    fn insert_block(&self, row: &CommandBlockRow) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO command_blocks
             (id, pane_id, session_id, command, exit_code, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                encode_uuid(row.id.0),
                encode_uuid(row.pane_id.0),
                encode_uuid(row.session_id.0),
                &row.command,
                row.exit_code,
                encode_dt(row.started_at),
                row.finished_at.map(encode_dt),
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn get_block(&self, id: BlockId) -> Result<Option<CommandBlockRow>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id, pane_id, session_id, command, exit_code, started_at, finished_at
             FROM command_blocks WHERE id = ?1",
            rusqlite::params![encode_uuid(id.0)],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        );
        match result {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
            Ok((id_s, pane_s, sess_s, command, exit_code, started_s, finished_s)) => {
                Ok(Some(CommandBlockRow {
                    id: BlockId(decode_uuid(&id_s)?),
                    pane_id: PaneId(decode_uuid(&pane_s)?),
                    session_id: SessionId(decode_uuid(&sess_s)?),
                    command,
                    exit_code,
                    started_at: decode_dt(&started_s)?,
                    finished_at: finished_s.map(|s| decode_dt(&s)).transpose()?,
                }))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AgentRunRepo
// ---------------------------------------------------------------------------

impl AgentRunRepo for SqliteStore {
    fn insert_run(&self, row: &AgentRunRow) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO agent_runs
             (id, session_id, adapter_name, started_at, exit_code)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                encode_uuid(row.id.0),
                encode_uuid(row.session_id.0),
                &row.adapter_name,
                encode_dt(row.started_at),
                row.exit_code,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn get_run(&self, id: AgentRunId) -> Result<Option<AgentRunRow>, StorageError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT id, session_id, adapter_name, started_at, exit_code
             FROM agent_runs WHERE id = ?1",
            rusqlite::params![encode_uuid(id.0)],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            },
        );
        match result {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
            Ok((id_s, sess_s, adapter, started_s, exit_code)) => Ok(Some(AgentRunRow {
                id: AgentRunId(decode_uuid(&id_s)?),
                session_id: SessionId(decode_uuid(&sess_s)?),
                adapter_name: adapter,
                started_at: decode_dt(&started_s)?,
                exit_code,
            })),
        }
    }
}

// ---------------------------------------------------------------------------
// TranscriptRepo
// ---------------------------------------------------------------------------

impl TranscriptRepo for SqliteStore {
    // chunk_index is a non-negative monotonic counter; SQLite stores INTEGER as
    // i64, so the u64->i64 cast is in range for any realistic chunk count.
    #[allow(clippy::cast_possible_wrap)]
    fn append_chunk(&self, row: &TranscriptRow) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO transcripts (id, agent_run_id, chunk_index, text)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                encode_uuid(row.id.0),
                encode_uuid(row.agent_run_id.0),
                row.chunk_index as i64,
                &row.text,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn list_chunks(&self, run_id: AgentRunId) -> Result<Vec<TranscriptRow>, StorageError> {
        let raw: Vec<(String, String, i64, String)> = {
            let conn = self.conn.lock();
            let mut stmt = conn
                .prepare(
                    "SELECT id, agent_run_id, chunk_index, text
                     FROM transcripts WHERE agent_run_id = ?1
                     ORDER BY chunk_index ASC",
                )
                .map_err(db_err)?;
            stmt.query_map(rusqlite::params![encode_uuid(run_id.0)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(db_err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_err)?
        };

        raw.into_iter()
            .map(|(id_s, run_s, chunk_index, text)| {
                Ok(TranscriptRow {
                    id: TranscriptId(decode_uuid(&id_s)?),
                    agent_run_id: AgentRunId(decode_uuid(&run_s)?),
                    chunk_index: chunk_index as u64,
                    text,
                })
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// McpCallRepo
// ---------------------------------------------------------------------------

impl McpCallRepo for SqliteStore {
    // duration_ms is a non-negative elapsed-millis count; SQLite stores INTEGER
    // as i64, so the u64->i64 cast is in range for any realistic duration.
    #[allow(clippy::cast_possible_wrap)]
    fn insert_call(&self, row: &McpCallRow) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO mcp_calls
             (id, agent_run_id, tool_name, duration_ms, succeeded)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                encode_uuid(row.id.0),
                encode_uuid(row.agent_run_id.0),
                &row.tool_name,
                row.duration_ms as i64,
                i32::from(row.succeeded),
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn list_calls(&self, run_id: AgentRunId) -> Result<Vec<McpCallRow>, StorageError> {
        let raw: Vec<(String, String, String, i64, i32)> = {
            let conn = self.conn.lock();
            let mut stmt = conn
                .prepare(
                    "SELECT id, agent_run_id, tool_name, duration_ms, succeeded
                     FROM mcp_calls WHERE agent_run_id = ?1",
                )
                .map_err(db_err)?;
            stmt.query_map(rusqlite::params![encode_uuid(run_id.0)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i32>(4)?,
                ))
            })
            .map_err(db_err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_err)?
        };

        raw.into_iter()
            .map(|(id_s, run_s, tool, dur_ms, succ)| {
                Ok(McpCallRow {
                    id: McpCallId(decode_uuid(&id_s)?),
                    agent_run_id: AgentRunId(decode_uuid(&run_s)?),
                    tool_name: tool,
                    duration_ms: dur_ms as u64,
                    succeeded: succ != 0,
                })
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// LedgerRepo
// ---------------------------------------------------------------------------

impl LedgerRepo for SqliteStore {
    // rss_bytes is a non-negative process-memory counter; SQLite stores INTEGER
    // as i64, so the u64->i64 cast is in range on supported targets.
    #[allow(clippy::cast_possible_wrap)]
    fn record_sample(
        &self,
        ts: time::OffsetDateTime,
        rss_bytes: u64,
        cpu_percent: f32,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO ledger_samples (ts, rss_bytes, cpu_percent) VALUES (?1, ?2, ?3)",
            rusqlite::params![encode_dt(ts), rss_bytes as i64, f64::from(cpu_percent)],
        )
        .map_err(db_err)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_test_kit::conformance::storage_repository_conformance::{
        run_block_repo_conformance, run_pane_repo_conformance, run_session_repo_conformance,
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn nil() -> Uuid {
        Uuid::nil()
    }

    fn migrated() -> SqliteStore {
        let store = SqliteStore::open_in_memory().expect("open in-memory DB");
        store.run_migrations().expect("run migrations");
        store
    }

    // ---- MigrationRunner ----

    #[test]
    fn migration_applies_once() {
        let store = SqliteStore::open_in_memory().expect("open");
        let first = store.run_migrations().expect("first run");
        let second = store.run_migrations().expect("second run");
        assert_eq!(first, 1, "first run must apply 1 migration");
        assert_eq!(second, 0, "second run must be idempotent");
    }

    // ---- WorkspaceRepo ----

    #[test]
    fn workspace_insert_get_roundtrip() {
        let store = migrated();
        let row = WorkspaceRow {
            id: WorkspaceId(nil()),
            name: "test-project".to_owned(),
            root_path: "C:\\projects\\test".to_owned(),
        };
        store.insert_workspace(&row).expect("insert");
        let got = store.get_workspace(row.id).expect("get").expect("some");
        assert_eq!(got.name, row.name);
        assert_eq!(got.root_path, row.root_path);
        assert_eq!(got.id.0, row.id.0);
    }

    #[test]
    fn workspace_not_found_returns_none() {
        let store = migrated();
        let result = store
            .get_workspace(WorkspaceId(Uuid::new_v4()))
            .expect("no error");
        assert!(result.is_none());
    }

    // ---- SessionRepo ----

    #[test]
    fn session_insert_get_roundtrip() {
        let store = migrated();
        let row = SessionRow {
            id: SessionId(nil()),
            workspace_id: WorkspaceId(nil()),
            started_at: OffsetDateTime::UNIX_EPOCH,
        };
        store.insert_session(&row).expect("insert");
        let got = store.get_session(row.id).expect("get").expect("some");
        assert_eq!(got.id.0, row.id.0);
        assert_eq!(got.workspace_id.0, row.workspace_id.0);
        assert_eq!(got.started_at, row.started_at);
    }

    // ---- PaneRepo ----

    #[test]
    fn pane_insert_get_roundtrip() {
        let store = migrated();
        let row = PaneRow {
            id: PaneId(nil()),
            session_id: SessionId(nil()),
            title: "main".to_owned(),
            cwd: "C:\\".to_owned(),
        };
        store.insert_pane(&row).expect("insert");
        let got = store.get_pane(row.id).expect("get").expect("some");
        assert_eq!(got.title, row.title);
        assert_eq!(got.cwd, row.cwd);
    }

    // ---- BlockRepo ----

    #[test]
    fn block_insert_get_roundtrip() {
        let store = migrated();
        let row = CommandBlockRow {
            id: BlockId(nil()),
            pane_id: PaneId(nil()),
            session_id: SessionId(nil()),
            command: "cargo test".to_owned(),
            exit_code: Some(0),
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        };
        store.insert_block(&row).expect("insert");
        let got = store.get_block(row.id).expect("get").expect("some");
        assert_eq!(got.command, row.command);
        assert_eq!(got.exit_code, row.exit_code);
        assert_eq!(got.started_at, row.started_at);
        assert_eq!(got.finished_at, row.finished_at);
    }

    #[test]
    fn block_null_exit_code_and_finished_at() {
        let store = migrated();
        let row = CommandBlockRow {
            id: BlockId(nil()),
            pane_id: PaneId(nil()),
            session_id: SessionId(nil()),
            command: "sleep 10".to_owned(),
            exit_code: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: None,
        };
        store.insert_block(&row).expect("insert");
        let got = store.get_block(row.id).expect("get").expect("some");
        assert!(got.exit_code.is_none());
        assert!(got.finished_at.is_none());
    }

    #[test]
    fn block_not_found_returns_none() {
        let store = migrated();
        let result = store.get_block(BlockId(Uuid::new_v4())).expect("no error");
        assert!(result.is_none());
    }

    // ---- AgentRunRepo ----

    #[test]
    fn agent_run_insert_get_roundtrip() {
        let store = migrated();
        let row = AgentRunRow {
            id: AgentRunId(nil()),
            session_id: SessionId(nil()),
            adapter_name: "claude-code".to_owned(),
            started_at: OffsetDateTime::UNIX_EPOCH,
            exit_code: None,
        };
        store.insert_run(&row).expect("insert");
        let got = store.get_run(row.id).expect("get").expect("some");
        assert_eq!(got.adapter_name, row.adapter_name);
        assert!(got.exit_code.is_none());
    }

    // ---- TranscriptRepo ----

    #[test]
    fn transcript_append_list_roundtrip() {
        let store = migrated();
        let run_id = AgentRunId(nil());
        for i in 0u64..3 {
            store
                .append_chunk(&TranscriptRow {
                    id: TranscriptId(Uuid::new_v4()),
                    agent_run_id: run_id,
                    chunk_index: i,
                    text: format!("chunk {i}"),
                })
                .expect("append");
        }
        let chunks = store.list_chunks(run_id).expect("list");
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].text, "chunk 1");
        assert_eq!(chunks[2].chunk_index, 2);
    }

    #[test]
    fn transcript_list_empty_for_unknown_run() {
        let store = migrated();
        let chunks = store.list_chunks(AgentRunId(Uuid::new_v4())).expect("list");
        assert!(chunks.is_empty());
    }

    // ---- McpCallRepo ----

    #[test]
    fn mcp_calls_insert_list_roundtrip() {
        let store = migrated();
        let run_id = AgentRunId(nil());
        let call = McpCallRow {
            id: McpCallId(nil()),
            agent_run_id: run_id,
            tool_name: "read_file".to_owned(),
            duration_ms: 42,
            succeeded: true,
        };
        store.insert_call(&call).expect("insert");
        let calls = store.list_calls(run_id).expect("list");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "read_file");
        assert_eq!(calls[0].duration_ms, 42);
        assert!(calls[0].succeeded);
    }

    #[test]
    fn mcp_calls_empty_for_unknown_run() {
        let store = migrated();
        let calls = store.list_calls(AgentRunId(Uuid::new_v4())).expect("list");
        assert!(calls.is_empty());
    }

    // ---- LedgerRepo ----

    #[test]
    fn ledger_record_sample_succeeds() {
        let store = migrated();
        store
            .record_sample(OffsetDateTime::UNIX_EPOCH, 1024 * 1024, 5.0)
            .expect("record sample");
        // Record multiple samples — no uniqueness constraint.
        store
            .record_sample(OffsetDateTime::UNIX_EPOCH, 2 * 1024 * 1024, 10.5)
            .expect("second sample");
    }

    // ---- Conformance ----

    #[test]
    fn block_repo_conformance() {
        run_block_repo_conformance(&migrated());
    }

    #[test]
    fn pane_repo_conformance() {
        run_pane_repo_conformance(&migrated());
    }

    #[test]
    fn session_repo_conformance() {
        run_session_repo_conformance(&migrated());
    }
}
