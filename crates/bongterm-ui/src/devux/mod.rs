//! Pure Developer-UX view-model projections for the shell UI.
//!
//! This module maps `bongterm-devassist` snapshots into UI-ready rows and
//! clickable region models. No I/O, no process execution, no state transitions.

use bongterm_devassist::ai::CmdKView;
use bongterm_devassist::jobs::list::{JobListSnapshot, JobRowView};
use bongterm_devassist::patterns::matchers::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdKBanner {
    pub headline: String,
    pub preview: Option<String>,
    pub run_enabled: bool,
    pub unavailable: bool,
}

#[must_use]
pub fn cmdk_banner(view: &CmdKView) -> CmdKBanner {
    match view {
        CmdKView::Idle => CmdKBanner {
            headline: "Cmd-K: describe a command".to_string(),
            preview: None,
            run_enabled: false,
            unavailable: false,
        },
        CmdKView::Previewed { command } => CmdKBanner {
            headline: "Preview — press Run to execute".to_string(),
            preview: Some(command.clone()),
            run_enabled: true,
            unavailable: false,
        },
        CmdKView::Unavailable { reason } => CmdKBanner {
            headline: format!("AI assist unavailable: {reason}"),
            preview: None,
            run_enabled: false,
            unavailable: true,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPanelRow {
    pub label: String,
    pub status: String,
    pub is_terminal: bool,
}

#[must_use]
pub fn job_panel_rows(snapshot: &JobListSnapshot) -> Vec<JobPanelRow> {
    snapshot
        .rows
        .iter()
        .map(|r: &JobRowView| JobPanelRow {
            label: r.label.clone(),
            status: r.status_label.clone(),
            is_terminal: r.is_terminal,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClickableRegion {
    pub start: usize,
    pub end: usize,
}

#[must_use]
pub fn clickable_regions(spans: &[Span]) -> Vec<ClickableRegion> {
    spans
        .iter()
        .map(|s| ClickableRegion {
            start: s.start,
            end: s.end,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_devassist::patterns::matchers::Span;

    #[test]
    fn idle_banner_has_no_preview_and_run_disabled() {
        let b = cmdk_banner(&CmdKView::Idle);
        assert_eq!(b.preview, None);
        assert!(!b.run_enabled, "idle must never be runnable");
        assert!(!b.unavailable);
    }

    #[test]
    fn previewed_banner_carries_command_and_enables_run() {
        let b = cmdk_banner(&CmdKView::Previewed {
            command: "git status".to_string(),
        });
        assert_eq!(b.preview.as_deref(), Some("git status"));
        assert!(b.run_enabled);
    }

    #[test]
    fn unavailable_banner_disables_run_and_flags_unavailable() {
        let b = cmdk_banner(&CmdKView::Unavailable {
            reason: "claude not found".to_string(),
        });
        assert!(!b.run_enabled, "unavailable must never be runnable");
        assert!(b.unavailable);
        assert!(b.headline.contains("claude not found"));
    }

    #[test]
    fn job_rows_map_one_to_one_in_order() {
        let snap = JobListSnapshot {
            rows: vec![
                JobRowView {
                    label: "build".to_string(),
                    status_label: "running".to_string(),
                    is_terminal: false,
                },
                JobRowView {
                    label: "test".to_string(),
                    status_label: "succeeded".to_string(),
                    is_terminal: true,
                },
            ],
        };
        let rows = job_panel_rows(&snap);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "build");
        assert!(!rows[0].is_terminal);
        assert_eq!(rows[1].status, "succeeded");
        assert!(rows[1].is_terminal);
    }

    #[test]
    fn clickable_regions_preserve_offsets_and_order() {
        let spans = vec![Span { start: 0, end: 4 }, Span { start: 10, end: 22 }];
        let regions = clickable_regions(&spans);
        assert_eq!(regions.len(), 2);
        assert_eq!((regions[0].start, regions[0].end), (0, 4));
        assert_eq!((regions[1].start, regions[1].end), (10, 22));
    }
}
