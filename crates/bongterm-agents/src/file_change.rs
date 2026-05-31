//! File-change attribution from `git status --porcelain=v1`.
//!
//! Git is the source of truth (per CLAUDE.md). This module never mutates the
//! repo; it reads porcelain status, diffs snapshots taken around an agent
//! run, and attributes the delta to that run.

/// One changed path with its git status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub status: ChangeStatus,
}

/// Normalized git status code (closed set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Other,
}

/// Parse `git status --porcelain=v1` output into [`FileChange`] rows.
#[must_use]
pub fn parse_porcelain_v1(output: &str) -> Vec<FileChange> {
    let mut changes = Vec::new();
    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }
        let xy = &line[..2];
        let rest = line[3..].trim();
        let (status, path) = if xy.starts_with('R') {
            let new_path = rest.split(" -> ").nth(1).unwrap_or(rest);
            (ChangeStatus::Renamed, new_path.to_string())
        } else if xy == "??" {
            (ChangeStatus::Untracked, rest.to_string())
        } else if xy.contains('A') {
            (ChangeStatus::Added, rest.to_string())
        } else if xy.contains('D') {
            (ChangeStatus::Deleted, rest.to_string())
        } else if xy.contains('M') {
            (ChangeStatus::Modified, rest.to_string())
        } else {
            (ChangeStatus::Other, rest.to_string())
        };
        changes.push(FileChange { path, status });
    }
    changes
}

/// Return changes present in `after` but not in `before` (path+status).
#[must_use]
pub fn attribute_new_changes(before: &[FileChange], after: &[FileChange]) -> Vec<FileChange> {
    after
        .iter()
        .filter(|c| {
            !before
                .iter()
                .any(|b| b.path == c.path && b.status == c.status)
        })
        .cloned()
        .collect()
}

/// Tracks file changes for a working directory by snapshotting porcelain status.
pub struct GitPorcelainTracker {
    cwd: String,
}

impl GitPorcelainTracker {
    #[must_use]
    pub fn new(cwd: impl Into<String>) -> Self {
        Self { cwd: cwd.into() }
    }

    /// Snapshot using an injected runner (tests / alternate transports).
    pub fn snapshot_with(
        &self,
        runner: impl Fn(&str) -> Result<String, String>,
    ) -> Result<Vec<FileChange>, String> {
        runner(&self.cwd).map(|out| parse_porcelain_v1(&out))
    }

    /// Production snapshot: invokes `git status --porcelain=v1`.
    pub fn snapshot(&self) -> Result<Vec<FileChange>, String> {
        self.snapshot_with(|cwd| {
            let out = std::process::Command::new("git")
                .args(["status", "--porcelain=v1"])
                .current_dir(cwd)
                .output()
                .map_err(|e| e.to_string())?;
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).into_owned())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).into_owned())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v1_status_codes() {
        let out = " M src/lib.rs\n?? new.txt\nA  added.rs\n D gone.rs\nR  old.rs -> renamed.rs\n";
        let changes = parse_porcelain_v1(out);
        assert_eq!(changes.len(), 5);
        assert_eq!(changes[0].path, "src/lib.rs");
        assert_eq!(changes[0].status, ChangeStatus::Modified);
        assert_eq!(changes[1].status, ChangeStatus::Untracked);
        assert_eq!(changes[2].status, ChangeStatus::Added);
        assert_eq!(changes[3].status, ChangeStatus::Deleted);
        assert_eq!(changes[4].status, ChangeStatus::Renamed);
        assert_eq!(changes[4].path, "renamed.rs");
    }

    #[test]
    fn diff_between_snapshots_attributes_only_new_changes() {
        let before = parse_porcelain_v1(" M a.rs\n");
        let after = parse_porcelain_v1(" M a.rs\n?? b.rs\n");
        let attributed = attribute_new_changes(&before, &after);
        assert_eq!(attributed.len(), 1);
        assert_eq!(attributed[0].path, "b.rs");
    }

    #[test]
    fn tracker_uses_injected_runner() {
        let tracker = GitPorcelainTracker::new("C:\\repo");
        let snap = tracker.snapshot_with(|_cwd| Ok(" M x.rs\n".to_string()));
        assert_eq!(snap.unwrap().len(), 1);
    }

    #[test]
    fn tracker_surfaces_runner_error() {
        let tracker = GitPorcelainTracker::new("C:\\repo");
        let r = tracker.snapshot_with(|_cwd| Err("git not found".to_string()));
        assert!(r.is_err());
    }
}
