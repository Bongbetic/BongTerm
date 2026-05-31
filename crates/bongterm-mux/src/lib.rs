//! `BongTerm` pane/tab topology model.
//!
//! Owns pane/tab structure and focus state. Does NOT own PTY sessions,
//! rendering, or input routing. Session teardown is the caller's responsibility
//! via the `PaneId` set returned by [`MuxRouter::close_tab`].
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

// ─── IDs ────────────────────────────────────────────────────────────────────

/// Opaque identifier for a pane. Unique within an [`InMemoryMux`] instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(u64);

/// Opaque identifier for a tab. Unique within an [`InMemoryMux`] instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(u64);

// ─── DTOs ───────────────────────────────────────────────────────────────────

/// Position and size of a pane within the terminal window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    /// Row offset from the top of the window.
    pub top: u16,
    /// Column offset from the left of the window.
    pub left: u16,
    pub cols: u16,
    pub rows: u16,
}

/// Direction of a pane split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Side-by-side: original takes left half, new pane takes right half.
    Horizontal,
    /// Stacked: original takes top half, new pane takes bottom half.
    Vertical,
}

/// Snapshot of a single pane's state.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: PaneId,
    pub tab_id: TabId,
    pub rect: Rect,
}

/// Snapshot of a single tab's structure.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: TabId,
    /// Pane IDs in creation order.
    pub pane_ids: Vec<PaneId>,
    /// Pane currently receiving input focus.
    pub active_pane_id: PaneId,
}

/// Pane IDs freed when a tab is closed. Caller must stop the associated sessions.
#[derive(Debug)]
pub struct ClosedTab {
    pub pane_ids: Vec<PaneId>,
}

// ─── Layout snapshots ────────────────────────────────────────────────────────

/// Serializable snapshot of a single pane's geometry.
///
/// PTY session, working directory, and shell command are NOT included —
/// those cross module boundaries and are composed by `bongterm-app` in a
/// higher-level `WorkspaceSnapshot`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PaneSnapshot {
    pub rect: Rect,
}

/// Serializable snapshot of a single tab's pane structure and focus state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TabSnapshot {
    /// Pane geometries in creation order.
    pub panes: Vec<PaneSnapshot>,
    /// Index into `panes` for the active pane.
    /// Clamped to `panes.len() - 1` on restore; defaults to 0.
    #[serde(default)]
    pub active_pane_index: usize,
}

/// Portable snapshot of the full tab/pane topology.
///
/// `PaneId` and `TabId` values are ephemeral and are **not** preserved —
/// new IDs are minted on restore. Use [`RestoreResult`] to reconnect sessions.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LayoutSnapshot {
    /// Tabs in insertion order.
    pub tabs: Vec<TabSnapshot>,
    /// Index into `tabs` for the active tab. `None` if `tabs` is empty.
    #[serde(default)]
    pub active_tab_index: Option<usize>,
}

/// Returned by [`MuxRouter::restore`] with the freshly-minted IDs.
///
/// Indices mirror the snapshot:
/// - `tab_ids[i]` ↔ `snapshot.tabs[i]`
/// - `pane_ids[i][j]` ↔ `snapshot.tabs[i].panes[j]`
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// New `TabId`s in order matching the snapshot's `tabs`.
    pub tab_ids: Vec<TabId>,
    /// New `PaneId`s per tab, in order matching each tab's `panes`.
    pub pane_ids: Vec<Vec<PaneId>>,
    /// Active tab after restore (`None` if the snapshot was empty).
    pub active_tab_id: Option<TabId>,
}

// ─── LayoutRepo port ─────────────────────────────────────────────────────────

/// Port for persisting a [`LayoutSnapshot`] to a backing store.
///
/// Real implementation: [`FileLayoutRepo`].
/// Test double: [`MockLayoutRepo`].
pub trait LayoutRepo: Send + Sync {
    /// Serialize and persist `snapshot`.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutRepoError`] on I/O or serialization failure.
    fn save(&self, snapshot: &LayoutSnapshot) -> Result<(), LayoutRepoError>;

    /// Load the most recently saved snapshot, or `None` if none exists.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutRepoError`] on I/O or deserialization failure.
    fn load(&self) -> Result<Option<LayoutSnapshot>, LayoutRepoError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutRepoError {
    #[error("layout I/O failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("layout JSON parse failed: {source}")]
    Parse {
        #[source]
        source: serde_json::Error,
    },
}

/// File-backed [`LayoutRepo`] implementation.
///
/// Saves as pretty-printed JSON using an atomic write (tmp→rename).
pub struct FileLayoutRepo {
    path: std::path::PathBuf,
}

impl FileLayoutRepo {
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl LayoutRepo for FileLayoutRepo {
    fn save(&self, snapshot: &LayoutSnapshot) -> Result<(), LayoutRepoError> {
        let json = serde_json::to_string_pretty(snapshot).expect("LayoutSnapshot must serialize");
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json.as_bytes()).map_err(|source| LayoutRepoError::Io {
            path: tmp.clone(),
            source,
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|source| LayoutRepoError::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(())
    }

    fn load(&self) -> Result<Option<LayoutSnapshot>, LayoutRepoError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&self.path).map_err(|source| LayoutRepoError::Io {
            path: self.path.clone(),
            source,
        })?;
        let snapshot =
            serde_json::from_str(&json).map_err(|source| LayoutRepoError::Parse { source })?;
        Ok(Some(snapshot))
    }
}

/// Call-recording test double for [`LayoutRepo`].
pub struct MockLayoutRepo {
    save_calls: Arc<parking_lot::Mutex<Vec<LayoutSnapshot>>>,
    stored: Arc<parking_lot::Mutex<Option<LayoutSnapshot>>>,
}

impl MockLayoutRepo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            save_calls: Arc::new(parking_lot::Mutex::new(Vec::new())),
            stored: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    /// All snapshots passed to [`LayoutRepo::save`] in call order.
    #[must_use]
    pub fn save_calls(&self) -> Vec<LayoutSnapshot> {
        self.save_calls.lock().clone()
    }

    /// Pre-load a snapshot that [`LayoutRepo::load`] will return.
    pub fn set_stored(&self, snapshot: LayoutSnapshot) {
        *self.stored.lock() = Some(snapshot);
    }
}

impl Default for MockLayoutRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutRepo for MockLayoutRepo {
    fn save(&self, snapshot: &LayoutSnapshot) -> Result<(), LayoutRepoError> {
        let mut calls = self.save_calls.lock();
        calls.push(snapshot.clone());
        *self.stored.lock() = Some(snapshot.clone());
        Ok(())
    }

    fn load(&self) -> Result<Option<LayoutSnapshot>, LayoutRepoError> {
        Ok(self.stored.lock().clone())
    }
}

// ─── Error ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum MuxError {
    #[error("tab not found: {0:?}")]
    TabNotFound(TabId),
    #[error("pane not found: {0:?}")]
    PaneNotFound(PaneId),
    #[error("pane {id:?} too small to split {direction:?}: {cols}×{rows}")]
    PaneTooSmallToSplit {
        id: PaneId,
        direction: SplitDirection,
        cols: u16,
        rows: u16,
    },
}

// ─── Trait ──────────────────────────────────────────────────────────────────

/// Port interface for the pane/tab topology model.
///
/// Real implementation: [`InMemoryMux`].
/// Test double: [`MockMuxRouter`].
/// Wired by `bongterm-app`; never called from hot-path code.
pub trait MuxRouter: Send + Sync {
    /// Open a new full-size tab with one pane and make it active.
    fn create_tab(&self, cols: u16, rows: u16) -> TabId;

    /// Close a tab, removing all its panes.
    ///
    /// Returns the `PaneId`s that were freed so the caller can stop sessions.
    /// If the closed tab was active, another open tab (if any) becomes active.
    ///
    /// # Errors
    ///
    /// Returns [`MuxError::TabNotFound`] if `id` is not open.
    fn close_tab(&self, id: TabId) -> Result<ClosedTab, MuxError>;

    /// The currently focused tab, or `None` if no tabs are open.
    fn active_tab_id(&self) -> Option<TabId>;

    /// Make an existing tab the active tab.
    ///
    /// # Errors
    ///
    /// Returns [`MuxError::TabNotFound`] if `id` is not open.
    fn set_active_tab(&self, id: TabId) -> Result<(), MuxError>;

    /// All open tab IDs in insertion order.
    fn tab_ids(&self) -> Vec<TabId>;

    /// Snapshot of a tab's pane structure and focus state.
    ///
    /// # Errors
    ///
    /// Returns [`MuxError::TabNotFound`] if `id` is not open.
    fn tab_info(&self, id: TabId) -> Result<TabInfo, MuxError>;

    /// Snapshot of a pane's position and size.
    ///
    /// # Errors
    ///
    /// Returns [`MuxError::PaneNotFound`] if `id` is not open.
    fn pane_info(&self, id: PaneId) -> Result<PaneInfo, MuxError>;

    /// Update a pane's stored rect (called on window resize or split drag).
    ///
    /// # Errors
    ///
    /// Returns [`MuxError::PaneNotFound`] if `id` is not open.
    fn resize_pane(&self, id: PaneId, rect: Rect) -> Result<(), MuxError>;

    /// Split a pane in half, returning the ID of the newly created sibling.
    ///
    /// The original pane's rect shrinks to the first half; the new pane
    /// occupies the second half. Original pane gets the extra row/col for
    /// odd dimensions. Focus stays on the original pane.
    ///
    /// # Errors
    ///
    /// - [`MuxError::PaneNotFound`] if `id` is not open.
    /// - [`MuxError::PaneTooSmallToSplit`] if the pane has fewer than 2 cols
    ///   (horizontal) or 2 rows (vertical).
    fn split_pane(&self, id: PaneId, direction: SplitDirection) -> Result<PaneId, MuxError>;

    /// Advance focus to the next pane in the tab, wrapping around.
    ///
    /// Returns the `PaneId` that is now active.
    ///
    /// # Errors
    ///
    /// Returns [`MuxError::TabNotFound`] if `tab_id` is not open.
    fn focus_next_pane(&self, tab_id: TabId) -> Result<PaneId, MuxError>;

    /// Capture the current tab/pane topology as a portable snapshot.
    ///
    /// Ephemeral IDs (`TabId`, `PaneId`) are **not** included. Use
    /// [`MuxRouter::restore`] to recreate the topology and obtain fresh IDs.
    fn snapshot(&self) -> LayoutSnapshot;

    /// Replace the current topology with `snapshot`.
    ///
    /// All existing tabs and panes are cleared; new IDs are minted.
    /// Returns [`RestoreResult`] with the new IDs so the caller can
    /// reconnect PTY sessions.
    fn restore(&self, snapshot: &LayoutSnapshot) -> RestoreResult;
}

// ─── InMemoryMux ────────────────────────────────────────────────────────────

struct TabEntry {
    pane_ids: Vec<PaneId>,
    active_pane_id: PaneId,
}

struct PaneEntry {
    tab_id: TabId,
    rect: Rect,
}

struct MuxState {
    next_id: u64,
    tabs: HashMap<TabId, TabEntry>,
    panes: HashMap<PaneId, PaneEntry>,
    tab_order: Vec<TabId>,
    active_tab: Option<TabId>,
}

impl MuxState {
    fn new() -> Self {
        Self {
            next_id: 1,
            tabs: HashMap::new(),
            panes: HashMap::new(),
            tab_order: Vec::new(),
            active_tab: None,
        }
    }

    fn alloc_tab_id(&mut self) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;
        id
    }

    fn alloc_pane_id(&mut self) -> PaneId {
        let id = PaneId(self.next_id);
        self.next_id += 1;
        id
    }
}

/// In-process pane/tab topology store. The canonical [`MuxRouter`] implementation.
pub struct InMemoryMux {
    state: Arc<RwLock<MuxState>>,
}

impl InMemoryMux {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(MuxState::new())),
        }
    }
}

impl Default for InMemoryMux {
    fn default() -> Self {
        Self::new()
    }
}

impl MuxRouter for InMemoryMux {
    fn create_tab(&self, cols: u16, rows: u16) -> TabId {
        let mut s = self.state.write();
        let tab_id = s.alloc_tab_id();
        let pane_id = s.alloc_pane_id();
        s.panes.insert(
            pane_id,
            PaneEntry {
                tab_id,
                rect: Rect {
                    top: 0,
                    left: 0,
                    cols,
                    rows,
                },
            },
        );
        s.tabs.insert(
            tab_id,
            TabEntry {
                pane_ids: vec![pane_id],
                active_pane_id: pane_id,
            },
        );
        s.tab_order.push(tab_id);
        s.active_tab = Some(tab_id);
        tab_id
    }

    fn close_tab(&self, id: TabId) -> Result<ClosedTab, MuxError> {
        let mut s = self.state.write();
        let entry = s.tabs.remove(&id).ok_or(MuxError::TabNotFound(id))?;
        s.tab_order.retain(|&t| t != id);
        for &pane_id in &entry.pane_ids {
            s.panes.remove(&pane_id);
        }
        if s.active_tab == Some(id) {
            s.active_tab = s.tab_order.last().copied();
        }
        Ok(ClosedTab {
            pane_ids: entry.pane_ids,
        })
    }

    fn active_tab_id(&self) -> Option<TabId> {
        self.state.read().active_tab
    }

    fn set_active_tab(&self, id: TabId) -> Result<(), MuxError> {
        let mut s = self.state.write();
        if !s.tabs.contains_key(&id) {
            return Err(MuxError::TabNotFound(id));
        }
        s.active_tab = Some(id);
        Ok(())
    }

    fn tab_ids(&self) -> Vec<TabId> {
        self.state.read().tab_order.clone()
    }

    fn tab_info(&self, id: TabId) -> Result<TabInfo, MuxError> {
        let s = self.state.read();
        let e = s.tabs.get(&id).ok_or(MuxError::TabNotFound(id))?;
        Ok(TabInfo {
            id,
            pane_ids: e.pane_ids.clone(),
            active_pane_id: e.active_pane_id,
        })
    }

    fn pane_info(&self, id: PaneId) -> Result<PaneInfo, MuxError> {
        let s = self.state.read();
        let e = s.panes.get(&id).ok_or(MuxError::PaneNotFound(id))?;
        Ok(PaneInfo {
            id,
            tab_id: e.tab_id,
            rect: e.rect,
        })
    }

    fn resize_pane(&self, id: PaneId, rect: Rect) -> Result<(), MuxError> {
        let mut s = self.state.write();
        let e = s.panes.get_mut(&id).ok_or(MuxError::PaneNotFound(id))?;
        e.rect = rect;
        Ok(())
    }

    fn split_pane(&self, id: PaneId, direction: SplitDirection) -> Result<PaneId, MuxError> {
        let mut s = self.state.write();

        let (rect, tab_id) = {
            let e = s.panes.get(&id).ok_or(MuxError::PaneNotFound(id))?;
            (e.rect, e.tab_id)
        };

        match direction {
            SplitDirection::Horizontal if rect.cols < 2 => {
                return Err(MuxError::PaneTooSmallToSplit {
                    id,
                    direction,
                    cols: rect.cols,
                    rows: rect.rows,
                });
            }
            SplitDirection::Vertical if rect.rows < 2 => {
                return Err(MuxError::PaneTooSmallToSplit {
                    id,
                    direction,
                    cols: rect.cols,
                    rows: rect.rows,
                });
            }
            _ => {}
        }

        let (orig_rect, new_rect) = match direction {
            SplitDirection::Horizontal => {
                let orig_cols = rect.cols.div_ceil(2);
                let new_cols = rect.cols - orig_cols;
                (
                    Rect {
                        top: rect.top,
                        left: rect.left,
                        cols: orig_cols,
                        rows: rect.rows,
                    },
                    Rect {
                        top: rect.top,
                        left: rect.left + orig_cols,
                        cols: new_cols,
                        rows: rect.rows,
                    },
                )
            }
            SplitDirection::Vertical => {
                let orig_rows = rect.rows.div_ceil(2);
                let new_rows = rect.rows - orig_rows;
                (
                    Rect {
                        top: rect.top,
                        left: rect.left,
                        cols: rect.cols,
                        rows: orig_rows,
                    },
                    Rect {
                        top: rect.top + orig_rows,
                        left: rect.left,
                        cols: rect.cols,
                        rows: new_rows,
                    },
                )
            }
        };

        s.panes.get_mut(&id).unwrap().rect = orig_rect;

        let new_pane_id = s.alloc_pane_id();
        s.panes.insert(
            new_pane_id,
            PaneEntry {
                tab_id,
                rect: new_rect,
            },
        );
        s.tabs.get_mut(&tab_id).unwrap().pane_ids.push(new_pane_id);

        Ok(new_pane_id)
    }

    fn focus_next_pane(&self, tab_id: TabId) -> Result<PaneId, MuxError> {
        let mut s = self.state.write();
        let tab = s
            .tabs
            .get_mut(&tab_id)
            .ok_or(MuxError::TabNotFound(tab_id))?;
        let current = tab.active_pane_id;
        let idx = tab.pane_ids.iter().position(|&p| p == current).unwrap_or(0);
        let next = tab.pane_ids[(idx + 1) % tab.pane_ids.len()];
        tab.active_pane_id = next;
        Ok(next)
    }

    fn snapshot(&self) -> LayoutSnapshot {
        let s = self.state.read();
        let active_tab_index = s
            .active_tab
            .and_then(|active| s.tab_order.iter().position(|&t| t == active));
        let tabs = s
            .tab_order
            .iter()
            .map(|&tab_id| {
                let entry = &s.tabs[&tab_id];
                let panes = entry
                    .pane_ids
                    .iter()
                    .map(|&pane_id| PaneSnapshot {
                        rect: s.panes[&pane_id].rect,
                    })
                    .collect();
                let active_pane_index = entry
                    .pane_ids
                    .iter()
                    .position(|&p| p == entry.active_pane_id)
                    .unwrap_or(0);
                TabSnapshot {
                    panes,
                    active_pane_index,
                }
            })
            .collect();
        LayoutSnapshot {
            tabs,
            active_tab_index,
        }
    }

    fn restore(&self, snapshot: &LayoutSnapshot) -> RestoreResult {
        let mut s = self.state.write();

        // Clear existing topology.
        s.tabs.clear();
        s.panes.clear();
        s.tab_order.clear();
        s.active_tab = None;

        let mut tab_ids: Vec<TabId> = Vec::with_capacity(snapshot.tabs.len());
        let mut pane_ids: Vec<Vec<PaneId>> = Vec::with_capacity(snapshot.tabs.len());

        for tab_snap in &snapshot.tabs {
            let tab_id = s.alloc_tab_id();
            let mut pane_ids_for_tab: Vec<PaneId> = Vec::with_capacity(tab_snap.panes.len());

            for pane_snap in &tab_snap.panes {
                let pane_id = s.alloc_pane_id();
                s.panes.insert(
                    pane_id,
                    PaneEntry {
                        tab_id,
                        rect: pane_snap.rect,
                    },
                );
                pane_ids_for_tab.push(pane_id);
            }

            // Clamp active_pane_index to valid range; default to first pane if empty.
            let active_pane_id = if pane_ids_for_tab.is_empty() {
                // Empty tab: no valid pane. Use id 0 as sentinel (never allocated).
                PaneId(0)
            } else {
                let idx = tab_snap.active_pane_index.min(pane_ids_for_tab.len() - 1);
                pane_ids_for_tab[idx]
            };

            s.tabs.insert(
                tab_id,
                TabEntry {
                    pane_ids: pane_ids_for_tab.clone(),
                    active_pane_id,
                },
            );
            s.tab_order.push(tab_id);
            tab_ids.push(tab_id);
            pane_ids.push(pane_ids_for_tab);
        }

        let active_tab_id = snapshot
            .active_tab_index
            .and_then(|i| tab_ids.get(i).copied());
        s.active_tab = active_tab_id;

        RestoreResult {
            tab_ids,
            pane_ids,
            active_tab_id,
        }
    }
}

// ─── MockMuxRouter ──────────────────────────────────────────────────────────

/// Call-recording spy for [`MuxRouter`]. Records method arguments for assertion
/// in higher-layer tests; delegates state operations to [`InMemoryMux`].
pub struct MockMuxRouter {
    delegate: InMemoryMux,
    calls: Arc<parking_lot::Mutex<MockCalls>>,
}

#[derive(Default)]
struct MockCalls {
    create_tab: Vec<(u16, u16)>,
    close_tab: Vec<TabId>,
    set_active_tab: Vec<TabId>,
    resize_pane: Vec<(PaneId, Rect)>,
    split_pane: Vec<(PaneId, SplitDirection)>,
    focus_next_pane: Vec<TabId>,
    restore: Vec<LayoutSnapshot>,
}

impl MockMuxRouter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            delegate: InMemoryMux::new(),
            calls: Arc::new(parking_lot::Mutex::new(MockCalls::default())),
        }
    }

    /// All `(cols, rows)` pairs passed to [`MuxRouter::create_tab`] in call order.
    #[must_use]
    pub fn create_tab_calls(&self) -> Vec<(u16, u16)> {
        self.calls.lock().create_tab.clone()
    }

    /// All `TabId`s passed to [`MuxRouter::close_tab`] in call order.
    #[must_use]
    pub fn close_tab_calls(&self) -> Vec<TabId> {
        self.calls.lock().close_tab.clone()
    }

    /// All `TabId`s passed to [`MuxRouter::set_active_tab`] in call order.
    #[must_use]
    pub fn set_active_tab_calls(&self) -> Vec<TabId> {
        self.calls.lock().set_active_tab.clone()
    }

    /// All `(PaneId, Rect)` pairs passed to [`MuxRouter::resize_pane`] in call order.
    #[must_use]
    pub fn resize_pane_calls(&self) -> Vec<(PaneId, Rect)> {
        self.calls.lock().resize_pane.clone()
    }

    /// All `(PaneId, SplitDirection)` pairs passed to [`MuxRouter::split_pane`] in call order.
    #[must_use]
    pub fn split_pane_calls(&self) -> Vec<(PaneId, SplitDirection)> {
        self.calls.lock().split_pane.clone()
    }

    /// All `TabId`s passed to [`MuxRouter::focus_next_pane`] in call order.
    #[must_use]
    pub fn focus_next_pane_calls(&self) -> Vec<TabId> {
        self.calls.lock().focus_next_pane.clone()
    }

    /// All [`LayoutSnapshot`]s passed to [`MuxRouter::restore`] in call order.
    #[must_use]
    pub fn restore_calls(&self) -> Vec<LayoutSnapshot> {
        self.calls.lock().restore.clone()
    }
}

impl Default for MockMuxRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MuxRouter for MockMuxRouter {
    fn create_tab(&self, cols: u16, rows: u16) -> TabId {
        self.calls.lock().create_tab.push((cols, rows));
        self.delegate.create_tab(cols, rows)
    }

    fn close_tab(&self, id: TabId) -> Result<ClosedTab, MuxError> {
        self.calls.lock().close_tab.push(id);
        self.delegate.close_tab(id)
    }

    fn active_tab_id(&self) -> Option<TabId> {
        self.delegate.active_tab_id()
    }

    fn set_active_tab(&self, id: TabId) -> Result<(), MuxError> {
        self.calls.lock().set_active_tab.push(id);
        self.delegate.set_active_tab(id)
    }

    fn tab_ids(&self) -> Vec<TabId> {
        self.delegate.tab_ids()
    }

    fn tab_info(&self, id: TabId) -> Result<TabInfo, MuxError> {
        self.delegate.tab_info(id)
    }

    fn pane_info(&self, id: PaneId) -> Result<PaneInfo, MuxError> {
        self.delegate.pane_info(id)
    }

    fn resize_pane(&self, id: PaneId, rect: Rect) -> Result<(), MuxError> {
        self.calls.lock().resize_pane.push((id, rect));
        self.delegate.resize_pane(id, rect)
    }

    fn split_pane(&self, id: PaneId, direction: SplitDirection) -> Result<PaneId, MuxError> {
        self.calls.lock().split_pane.push((id, direction));
        self.delegate.split_pane(id, direction)
    }

    fn focus_next_pane(&self, tab_id: TabId) -> Result<PaneId, MuxError> {
        self.calls.lock().focus_next_pane.push(tab_id);
        self.delegate.focus_next_pane(tab_id)
    }

    fn snapshot(&self) -> LayoutSnapshot {
        self.delegate.snapshot()
    }

    fn restore(&self, snapshot: &LayoutSnapshot) -> RestoreResult {
        self.calls.lock().restore.push(snapshot.clone());
        self.delegate.restore(snapshot)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- Type invariants ---

    #[test]
    fn tab_id_distinct_across_creates() {
        let mux = InMemoryMux::new();
        let a = mux.create_tab(80, 24);
        let b = mux.create_tab(80, 24);
        assert_ne!(a, b);
    }

    #[test]
    fn pane_id_distinct_across_creates() {
        let mux = InMemoryMux::new();
        let a = mux.create_tab(80, 24);
        let b = mux.create_tab(80, 24);
        let pane_a = mux.tab_info(a).unwrap().pane_ids[0];
        let pane_b = mux.tab_info(b).unwrap().pane_ids[0];
        assert_ne!(pane_a, pane_b);
    }

    // --- create_tab ---

    #[test]
    fn create_tab_sets_active() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        assert_eq!(mux.active_tab_id(), Some(id));
    }

    #[test]
    fn create_tab_replaces_active() {
        let mux = InMemoryMux::new();
        let _first = mux.create_tab(80, 24);
        let second = mux.create_tab(80, 24);
        assert_eq!(mux.active_tab_id(), Some(second));
    }

    #[test]
    fn create_tab_has_one_pane() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        assert_eq!(mux.tab_info(id).unwrap().pane_ids.len(), 1);
    }

    #[test]
    fn create_tab_active_pane_matches_only_pane() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        let info = mux.tab_info(id).unwrap();
        assert_eq!(info.active_pane_id, info.pane_ids[0]);
    }

    #[test]
    fn tab_ids_in_insertion_order() {
        let mux = InMemoryMux::new();
        let a = mux.create_tab(80, 24);
        let b = mux.create_tab(80, 24);
        let c = mux.create_tab(80, 24);
        assert_eq!(mux.tab_ids(), vec![a, b, c]);
    }

    #[test]
    fn no_active_tab_when_none_created() {
        let mux = InMemoryMux::new();
        assert_eq!(mux.active_tab_id(), None);
    }

    // --- pane_info ---

    #[test]
    fn pane_rect_matches_creation() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(120, 40);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        assert_eq!(
            mux.pane_info(pane_id).unwrap().rect,
            Rect {
                top: 0,
                left: 0,
                cols: 120,
                rows: 40
            }
        );
    }

    #[test]
    fn pane_info_links_back_to_tab() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        assert_eq!(mux.pane_info(pane_id).unwrap().tab_id, tab_id);
    }

    // --- close_tab ---

    #[test]
    fn close_tab_removes_from_list() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        mux.close_tab(id).unwrap();
        assert!(!mux.tab_ids().contains(&id));
    }

    #[test]
    fn close_tab_returns_pane_ids() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let closed = mux.close_tab(tab_id).unwrap();
        assert_eq!(closed.pane_ids, vec![pane_id]);
    }

    #[test]
    fn close_last_tab_clears_active() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        mux.close_tab(id).unwrap();
        assert_eq!(mux.active_tab_id(), None);
    }

    #[test]
    fn close_active_tab_switches_to_another() {
        let mux = InMemoryMux::new();
        let a = mux.create_tab(80, 24);
        let _b = mux.create_tab(80, 24);
        let _c = mux.create_tab(80, 24);
        let closed_id = mux.active_tab_id().unwrap();
        mux.close_tab(closed_id).unwrap();
        let new_active = mux.active_tab_id().expect("another tab should be active");
        assert_ne!(new_active, closed_id);
        assert!(mux.tab_ids().contains(&new_active));
        assert!(mux.tab_ids().contains(&a));
    }

    #[test]
    fn close_non_active_tab_preserves_active() {
        let mux = InMemoryMux::new();
        let a = mux.create_tab(80, 24);
        let b = mux.create_tab(80, 24);
        mux.close_tab(a).unwrap();
        assert_eq!(mux.active_tab_id(), Some(b));
    }

    #[test]
    fn closed_pane_removed_from_store() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.close_tab(tab_id).unwrap();
        assert!(matches!(
            mux.pane_info(pane_id),
            Err(MuxError::PaneNotFound(_))
        ));
    }

    // --- set_active_tab ---

    #[test]
    fn set_active_tab_changes_focus() {
        let mux = InMemoryMux::new();
        let a = mux.create_tab(80, 24);
        let _b = mux.create_tab(80, 24);
        mux.set_active_tab(a).unwrap();
        assert_eq!(mux.active_tab_id(), Some(a));
    }

    // --- resize_pane ---

    #[test]
    fn resize_pane_updates_rect() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let new_rect = Rect {
            top: 0,
            left: 0,
            cols: 132,
            rows: 50,
        };
        mux.resize_pane(pane_id, new_rect).unwrap();
        assert_eq!(mux.pane_info(pane_id).unwrap().rect, new_rect);
    }

    // --- split_pane ---

    #[test]
    fn horizontal_split_creates_second_pane() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let new_pane = mux.split_pane(orig, SplitDirection::Horizontal).unwrap();
        assert_ne!(orig, new_pane);
        assert_eq!(mux.tab_info(tab_id).unwrap().pane_ids.len(), 2);
    }

    #[test]
    fn vertical_split_creates_second_pane() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let new_pane = mux.split_pane(orig, SplitDirection::Vertical).unwrap();
        assert_ne!(orig, new_pane);
        assert_eq!(mux.tab_info(tab_id).unwrap().pane_ids.len(), 2);
    }

    #[test]
    fn horizontal_split_divides_cols_evenly() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let sibling = mux.split_pane(orig, SplitDirection::Horizontal).unwrap();

        let orig_rect = mux.pane_info(orig).unwrap().rect;
        let sibling_rect = mux.pane_info(sibling).unwrap().rect;

        assert_eq!(
            orig_rect,
            Rect {
                top: 0,
                left: 0,
                cols: 40,
                rows: 24
            }
        );
        assert_eq!(
            sibling_rect,
            Rect {
                top: 0,
                left: 40,
                cols: 40,
                rows: 24
            }
        );
    }

    #[test]
    fn vertical_split_divides_rows_evenly() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let sibling = mux.split_pane(orig, SplitDirection::Vertical).unwrap();

        let orig_rect = mux.pane_info(orig).unwrap().rect;
        let sibling_rect = mux.pane_info(sibling).unwrap().rect;

        assert_eq!(
            orig_rect,
            Rect {
                top: 0,
                left: 0,
                cols: 80,
                rows: 12
            }
        );
        assert_eq!(
            sibling_rect,
            Rect {
                top: 12,
                left: 0,
                cols: 80,
                rows: 12
            }
        );
    }

    #[test]
    fn horizontal_split_odd_cols_gives_extra_to_original() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(81, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let sibling = mux.split_pane(orig, SplitDirection::Horizontal).unwrap();

        assert_eq!(mux.pane_info(orig).unwrap().rect.cols, 41);
        assert_eq!(mux.pane_info(sibling).unwrap().rect.cols, 40);
    }

    #[test]
    fn vertical_split_odd_rows_gives_extra_to_original() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 25);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let sibling = mux.split_pane(orig, SplitDirection::Vertical).unwrap();

        assert_eq!(mux.pane_info(orig).unwrap().rect.rows, 13);
        assert_eq!(mux.pane_info(sibling).unwrap().rect.rows, 12);
    }

    #[test]
    fn split_new_pane_belongs_to_same_tab() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let sibling = mux.split_pane(orig, SplitDirection::Horizontal).unwrap();
        assert_eq!(mux.pane_info(sibling).unwrap().tab_id, tab_id);
    }

    #[test]
    fn split_focus_stays_on_original() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.split_pane(orig, SplitDirection::Horizontal).unwrap();
        assert_eq!(mux.tab_info(tab_id).unwrap().active_pane_id, orig);
    }

    #[test]
    fn horizontal_split_too_narrow_returns_error() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(1, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        assert!(matches!(
            mux.split_pane(pane_id, SplitDirection::Horizontal),
            Err(MuxError::PaneTooSmallToSplit { .. })
        ));
    }

    #[test]
    fn vertical_split_too_short_returns_error() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 1);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        assert!(matches!(
            mux.split_pane(pane_id, SplitDirection::Vertical),
            Err(MuxError::PaneTooSmallToSplit { .. })
        ));
    }

    #[test]
    fn split_unknown_pane_returns_error() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.close_tab(tab_id).unwrap();
        assert!(matches!(
            mux.split_pane(pane_id, SplitDirection::Horizontal),
            Err(MuxError::PaneNotFound(_))
        ));
    }

    #[test]
    fn horizontal_split_minimum_width_succeeds() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(2, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        assert!(mux.split_pane(pane_id, SplitDirection::Horizontal).is_ok());
    }

    // --- focus_next_pane ---

    #[test]
    fn focus_next_pane_single_pane_returns_same() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let focused = mux.focus_next_pane(tab_id).unwrap();
        assert_eq!(focused, pane_id);
    }

    #[test]
    fn focus_next_pane_advances_to_sibling() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let sibling = mux.split_pane(orig, SplitDirection::Horizontal).unwrap();

        let focused = mux.focus_next_pane(tab_id).unwrap();
        assert_eq!(focused, sibling);
        assert_eq!(mux.tab_info(tab_id).unwrap().active_pane_id, sibling);
    }

    #[test]
    fn focus_next_pane_wraps_around() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.split_pane(orig, SplitDirection::Horizontal).unwrap();

        mux.focus_next_pane(tab_id).unwrap(); // orig → sibling
        let back = mux.focus_next_pane(tab_id).unwrap(); // sibling → orig (wrap)
        assert_eq!(back, orig);
    }

    #[test]
    fn focus_next_pane_unknown_tab_returns_error() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        mux.close_tab(tab_id).unwrap();
        assert!(matches!(
            mux.focus_next_pane(tab_id),
            Err(MuxError::TabNotFound(_))
        ));
    }

    // --- error paths ---

    #[test]
    fn tab_info_after_close_returns_error() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        mux.close_tab(id).unwrap();
        assert!(matches!(mux.tab_info(id), Err(MuxError::TabNotFound(_))));
    }

    #[test]
    fn close_already_closed_tab_returns_error() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        mux.close_tab(id).unwrap();
        assert!(matches!(mux.close_tab(id), Err(MuxError::TabNotFound(_))));
    }

    #[test]
    fn pane_info_after_tab_close_returns_error() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.close_tab(tab_id).unwrap();
        assert!(matches!(
            mux.pane_info(pane_id),
            Err(MuxError::PaneNotFound(_))
        ));
    }

    #[test]
    fn set_active_closed_tab_returns_error() {
        let mux = InMemoryMux::new();
        let id = mux.create_tab(80, 24);
        mux.close_tab(id).unwrap();
        assert!(matches!(
            mux.set_active_tab(id),
            Err(MuxError::TabNotFound(_))
        ));
    }

    #[test]
    fn resize_closed_pane_returns_error() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.close_tab(tab_id).unwrap();
        assert!(matches!(
            mux.resize_pane(
                pane_id,
                Rect {
                    top: 0,
                    left: 0,
                    cols: 80,
                    rows: 24
                }
            ),
            Err(MuxError::PaneNotFound(_))
        ));
    }

    // --- Send + Sync ---

    #[test]
    fn in_memory_mux_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemoryMux>();
        assert_send_sync::<MockMuxRouter>();
    }

    // --- MockMuxRouter call recording ---

    #[test]
    fn mock_records_create_tab_calls() {
        let mock = MockMuxRouter::new();
        mock.create_tab(80, 24);
        mock.create_tab(132, 50);
        assert_eq!(
            mock.create_tab_calls(),
            vec![(80u16, 24u16), (132u16, 50u16)]
        );
    }

    #[test]
    fn mock_records_close_tab_calls() {
        let mock = MockMuxRouter::new();
        let id = mock.create_tab(80, 24);
        mock.close_tab(id).unwrap();
        assert_eq!(mock.close_tab_calls(), vec![id]);
    }

    #[test]
    fn mock_records_set_active_tab_calls() {
        let mock = MockMuxRouter::new();
        let a = mock.create_tab(80, 24);
        let b = mock.create_tab(80, 24);
        mock.set_active_tab(a).unwrap();
        mock.set_active_tab(b).unwrap();
        assert_eq!(mock.set_active_tab_calls(), vec![a, b]);
    }

    #[test]
    fn mock_records_resize_pane_calls() {
        let mock = MockMuxRouter::new();
        let tab_id = mock.create_tab(80, 24);
        let pane_id = mock.tab_info(tab_id).unwrap().pane_ids[0];
        let new_rect = Rect {
            top: 0,
            left: 0,
            cols: 132,
            rows: 50,
        };
        mock.resize_pane(pane_id, new_rect).unwrap();
        assert_eq!(mock.resize_pane_calls(), vec![(pane_id, new_rect)]);
    }

    #[test]
    fn mock_records_split_pane_calls() {
        let mock = MockMuxRouter::new();
        let tab_id = mock.create_tab(80, 24);
        let pane_id = mock.tab_info(tab_id).unwrap().pane_ids[0];
        mock.split_pane(pane_id, SplitDirection::Horizontal)
            .unwrap();
        assert_eq!(
            mock.split_pane_calls(),
            vec![(pane_id, SplitDirection::Horizontal)]
        );
    }

    #[test]
    fn mock_records_focus_next_pane_calls() {
        let mock = MockMuxRouter::new();
        let tab_id = mock.create_tab(80, 24);
        mock.focus_next_pane(tab_id).unwrap();
        mock.focus_next_pane(tab_id).unwrap();
        assert_eq!(mock.focus_next_pane_calls(), vec![tab_id, tab_id]);
    }

    #[test]
    fn mock_state_is_live_after_calls() {
        let mock = MockMuxRouter::new();
        let tab_id = mock.create_tab(80, 24);
        assert_eq!(mock.active_tab_id(), Some(tab_id));
        mock.close_tab(tab_id).unwrap();
        assert_eq!(mock.active_tab_id(), None);
    }

    // --- snapshot ---

    #[test]
    fn snapshot_empty_mux_yields_empty_layout() {
        let mux = InMemoryMux::new();
        let snap = mux.snapshot();
        assert!(snap.tabs.is_empty());
        assert_eq!(snap.active_tab_index, None);
    }

    #[test]
    fn snapshot_single_tab_one_pane() {
        let mux = InMemoryMux::new();
        mux.create_tab(80, 24);
        let snap = mux.snapshot();
        assert_eq!(snap.tabs.len(), 1);
        assert_eq!(snap.tabs[0].panes.len(), 1);
        assert_eq!(
            snap.tabs[0].panes[0].rect,
            Rect {
                top: 0,
                left: 0,
                cols: 80,
                rows: 24
            }
        );
        assert_eq!(snap.tabs[0].active_pane_index, 0);
        assert_eq!(snap.active_tab_index, Some(0));
    }

    #[test]
    fn snapshot_active_tab_index_matches_current_active() {
        let mux = InMemoryMux::new();
        mux.create_tab(80, 24);
        let _b = mux.create_tab(80, 24);
        let c = mux.create_tab(80, 24);
        mux.set_active_tab(c).unwrap();
        // c is at index 2
        assert_eq!(mux.snapshot().active_tab_index, Some(2));
    }

    #[test]
    fn snapshot_active_pane_index_matches_focused_pane() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let orig = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.split_pane(orig, SplitDirection::Horizontal).unwrap();
        mux.focus_next_pane(tab_id).unwrap(); // focus moves to sibling (index 1)

        let snap = mux.snapshot();
        assert_eq!(snap.tabs[0].active_pane_index, 1);
    }

    #[test]
    fn snapshot_multiple_tabs_captures_all() {
        let mux = InMemoryMux::new();
        mux.create_tab(80, 24);
        mux.create_tab(132, 50);
        let snap = mux.snapshot();
        assert_eq!(snap.tabs.len(), 2);
        assert_eq!(snap.tabs[0].panes[0].rect.cols, 80);
        assert_eq!(snap.tabs[1].panes[0].rect.cols, 132);
    }

    // --- restore ---

    #[test]
    fn restore_empty_snapshot_clears_existing_state() {
        let mux = InMemoryMux::new();
        mux.create_tab(80, 24);
        let empty = LayoutSnapshot {
            tabs: vec![],
            active_tab_index: None,
        };
        let result = mux.restore(&empty);
        assert!(result.tab_ids.is_empty());
        assert_eq!(mux.active_tab_id(), None);
        assert!(mux.tab_ids().is_empty());
    }

    #[test]
    fn restore_recreates_single_tab_one_pane() {
        let mux = InMemoryMux::new();
        let snap = LayoutSnapshot {
            tabs: vec![TabSnapshot {
                panes: vec![PaneSnapshot {
                    rect: Rect {
                        top: 0,
                        left: 0,
                        cols: 80,
                        rows: 24,
                    },
                }],
                active_pane_index: 0,
            }],
            active_tab_index: Some(0),
        };
        let result = mux.restore(&snap);
        assert_eq!(result.tab_ids.len(), 1);
        assert_eq!(result.pane_ids[0].len(), 1);
        assert_eq!(result.active_tab_id, Some(result.tab_ids[0]));

        let pane_id = result.pane_ids[0][0];
        assert_eq!(
            mux.pane_info(pane_id).unwrap().rect,
            Rect {
                top: 0,
                left: 0,
                cols: 80,
                rows: 24
            }
        );
    }

    #[test]
    fn restore_sets_active_tab() {
        let mux = InMemoryMux::new();
        let snap = LayoutSnapshot {
            tabs: vec![
                TabSnapshot {
                    panes: vec![PaneSnapshot {
                        rect: Rect {
                            top: 0,
                            left: 0,
                            cols: 80,
                            rows: 24,
                        },
                    }],
                    active_pane_index: 0,
                },
                TabSnapshot {
                    panes: vec![PaneSnapshot {
                        rect: Rect {
                            top: 0,
                            left: 0,
                            cols: 132,
                            rows: 50,
                        },
                    }],
                    active_pane_index: 0,
                },
            ],
            active_tab_index: Some(1),
        };
        let result = mux.restore(&snap);
        assert_eq!(mux.active_tab_id(), Some(result.tab_ids[1]));
    }

    #[test]
    fn restore_sets_active_pane_within_tab() {
        let mux = InMemoryMux::new();
        let snap = LayoutSnapshot {
            tabs: vec![TabSnapshot {
                panes: vec![
                    PaneSnapshot {
                        rect: Rect {
                            top: 0,
                            left: 0,
                            cols: 40,
                            rows: 24,
                        },
                    },
                    PaneSnapshot {
                        rect: Rect {
                            top: 0,
                            left: 40,
                            cols: 40,
                            rows: 24,
                        },
                    },
                ],
                active_pane_index: 1,
            }],
            active_tab_index: Some(0),
        };
        let result = mux.restore(&snap);
        let tab_info = mux.tab_info(result.tab_ids[0]).unwrap();
        assert_eq!(tab_info.active_pane_id, result.pane_ids[0][1]);
    }

    #[test]
    fn restore_replaces_existing_state() {
        let mux = InMemoryMux::new();
        mux.create_tab(80, 24);
        mux.create_tab(80, 24);
        // Restore to single tab
        let snap = LayoutSnapshot {
            tabs: vec![TabSnapshot {
                panes: vec![PaneSnapshot {
                    rect: Rect {
                        top: 0,
                        left: 0,
                        cols: 100,
                        rows: 30,
                    },
                }],
                active_pane_index: 0,
            }],
            active_tab_index: Some(0),
        };
        mux.restore(&snap);
        assert_eq!(mux.tab_ids().len(), 1);
    }

    #[test]
    fn restore_mints_fresh_ids() {
        let mux = InMemoryMux::new();
        let original_tab = mux.create_tab(80, 24);

        let snap = mux.snapshot();
        let result = mux.restore(&snap);

        // New IDs must differ from the original ones
        assert_ne!(result.tab_ids[0], original_tab);
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mux = InMemoryMux::new();
        let tab1 = mux.create_tab(80, 24);
        let pane1 = mux.tab_info(tab1).unwrap().pane_ids[0];
        mux.split_pane(pane1, SplitDirection::Horizontal).unwrap();
        mux.create_tab(132, 50);

        let snap1 = mux.snapshot();
        mux.restore(&snap1);
        let snap2 = mux.snapshot();

        assert_eq!(snap1, snap2);
    }

    #[test]
    fn layout_snapshot_json_roundtrip() {
        let snap = LayoutSnapshot {
            tabs: vec![TabSnapshot {
                panes: vec![
                    PaneSnapshot {
                        rect: Rect {
                            top: 0,
                            left: 0,
                            cols: 40,
                            rows: 24,
                        },
                    },
                    PaneSnapshot {
                        rect: Rect {
                            top: 0,
                            left: 40,
                            cols: 40,
                            rows: 24,
                        },
                    },
                ],
                active_pane_index: 1,
            }],
            active_tab_index: Some(0),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let restored: LayoutSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap, restored);
    }

    // --- FileLayoutRepo ---

    #[test]
    fn file_layout_repo_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("layout.json");
        let repo = FileLayoutRepo::new(&path);

        let snap = LayoutSnapshot {
            tabs: vec![TabSnapshot {
                panes: vec![PaneSnapshot {
                    rect: Rect {
                        top: 0,
                        left: 0,
                        cols: 80,
                        rows: 24,
                    },
                }],
                active_pane_index: 0,
            }],
            active_tab_index: Some(0),
        };

        repo.save(&snap).unwrap();
        let loaded = repo.load().unwrap().expect("should load saved snapshot");
        assert_eq!(snap, loaded);
    }

    #[test]
    fn file_layout_repo_load_absent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let repo = FileLayoutRepo::new(&path);
        assert!(repo.load().unwrap().is_none());
    }

    #[test]
    fn file_layout_repo_save_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("layout.json");
        let repo = FileLayoutRepo::new(&path);
        let snap = LayoutSnapshot {
            tabs: vec![],
            active_tab_index: None,
        };
        repo.save(&snap).unwrap();
        assert!(path.exists());
    }

    // --- MockLayoutRepo ---

    #[test]
    fn mock_layout_repo_records_save_calls() {
        let mock = MockLayoutRepo::new();
        let snap1 = LayoutSnapshot {
            tabs: vec![],
            active_tab_index: None,
        };
        let snap2 = LayoutSnapshot {
            tabs: vec![TabSnapshot {
                panes: vec![PaneSnapshot {
                    rect: Rect {
                        top: 0,
                        left: 0,
                        cols: 80,
                        rows: 24,
                    },
                }],
                active_pane_index: 0,
            }],
            active_tab_index: Some(0),
        };
        mock.save(&snap1).unwrap();
        mock.save(&snap2).unwrap();
        assert_eq!(mock.save_calls().len(), 2);
    }

    #[test]
    fn mock_layout_repo_load_returns_none_initially() {
        let mock = MockLayoutRepo::new();
        assert!(mock.load().unwrap().is_none());
    }

    #[test]
    fn mock_layout_repo_set_stored_controls_load() {
        let mock = MockLayoutRepo::new();
        let snap = LayoutSnapshot {
            tabs: vec![],
            active_tab_index: None,
        };
        mock.set_stored(snap.clone());
        assert_eq!(mock.load().unwrap(), Some(snap));
    }

    // --- MockMuxRouter restore recording ---

    #[test]
    fn mock_mux_records_restore_calls() {
        let mock = MockMuxRouter::new();
        let snap = LayoutSnapshot {
            tabs: vec![],
            active_tab_index: None,
        };
        mock.restore(&snap);
        mock.restore(&snap);
        assert_eq!(mock.restore_calls().len(), 2);
    }
}
