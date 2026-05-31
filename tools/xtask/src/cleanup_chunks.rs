//! Remove orphaned sidecar chunk files that have no corresponding pane record
//! in the BongTerm SQLite database.
//!
//! ## Expected layout under the BongT app-data directory
//!
//! ```text
//! %APPDATA%\BongT\
//!   bongt.db          ← SQLite database
//!   chunks\           ← sidecar chunk files
//!     <pane-uuid>.bin
//!     ...
//! ```
//!
//! A chunk file is orphaned when its file stem (a pane UUID) does not appear
//! in the `panes` table.  The command is idempotent and safe to run while the
//! application is not running.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn run() -> Result<()> {
    let data_dir = app_data_dir()?;
    let db_path = data_dir.join("bongt.db");
    let chunks_dir = data_dir.join("chunks");

    if !db_path.exists() {
        println!("No database at {}; nothing to clean.", db_path.display());
        return Ok(());
    }
    if !chunks_dir.exists() {
        println!(
            "No chunks directory at {}; nothing to clean.",
            chunks_dir.display()
        );
        return Ok(());
    }

    let known = load_pane_ids(&db_path)?;
    let removed = remove_orphaned(&chunks_dir, &known)?;
    println!("Removed {removed} orphaned chunk file(s).");
    Ok(())
}

fn app_data_dir() -> Result<PathBuf> {
    let appdata = std::env::var("APPDATA").context("%APPDATA% not set")?;
    Ok(PathBuf::from(appdata).join("BongT"))
}

/// Query the `panes` table and return all pane IDs as strings.
///
/// Returns an empty set if the `panes` table does not exist (migrations not
/// yet run).
fn load_pane_ids(db_path: &Path) -> Result<HashSet<String>> {
    let conn = rusqlite::Connection::open(db_path)
        .with_context(|| format!("open DB at {}", db_path.display()))?;

    // Guard: if panes table hasn't been created yet, there are no valid IDs.
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='panes'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if table_exists == 0 {
        return Ok(HashSet::new());
    }

    let mut stmt = conn.prepare("SELECT id FROM panes")?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<HashSet<String>>>()
        .context("read pane IDs from DB")?;
    Ok(ids)
}

/// Walk `chunks_dir`, remove any `.bin` file whose stem is not in `known_ids`.
///
/// Returns the count of files removed.
fn remove_orphaned(chunks_dir: &Path, known_ids: &HashSet<String>) -> Result<usize> {
    let mut removed = 0usize;
    for entry in std::fs::read_dir(chunks_dir).context("read chunks dir")? {
        let entry = entry.context("read dir entry")?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "bin") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_owned();
        if !known_ids.contains(&stem) {
            std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
            println!("  removed orphan: {}", path.display());
            removed += 1;
        }
    }
    Ok(removed)
}
