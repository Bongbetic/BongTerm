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

/// Size of a pane within the window. `top`/`left` offsets are deferred to 1.D.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub cols: u16,
    pub rows: u16,
}

/// Snapshot of a single pane's state.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: PaneId,
    pub tab_id: TabId,
    pub geometry: Geometry,
}

/// Snapshot of a single tab's structure.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: TabId,
    /// Pane IDs in creation order (1.D.2 will extend this to split panes).
    pub pane_ids: Vec<PaneId>,
    /// Pane currently receiving input focus.
    pub active_pane_id: PaneId,
}

/// Pane IDs freed when a tab is closed. Caller must stop the associated sessions.
#[derive(Debug)]
pub struct ClosedTab {
    pub pane_ids: Vec<PaneId>,
}

// ─── Error ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum MuxError {
    #[error("tab not found: {0:?}")]
    TabNotFound(TabId),
    #[error("pane not found: {0:?}")]
    PaneNotFound(PaneId),
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

    /// Update a pane's stored geometry (called on window or split resize).
    ///
    /// # Errors
    ///
    /// Returns [`MuxError::PaneNotFound`] if `id` is not open.
    fn resize_pane(&self, id: PaneId, cols: u16, rows: u16) -> Result<(), MuxError>;
}

// ─── InMemoryMux ────────────────────────────────────────────────────────────

struct TabEntry {
    pane_ids: Vec<PaneId>,
    active_pane_id: PaneId,
}

struct PaneEntry {
    tab_id: TabId,
    geometry: Geometry,
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
                geometry: Geometry { cols, rows },
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
            geometry: e.geometry,
        })
    }

    fn resize_pane(&self, id: PaneId, cols: u16, rows: u16) -> Result<(), MuxError> {
        let mut s = self.state.write();
        let e = s.panes.get_mut(&id).ok_or(MuxError::PaneNotFound(id))?;
        e.geometry = Geometry { cols, rows };
        Ok(())
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
    resize_pane: Vec<(PaneId, u16, u16)>,
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

    /// All `(PaneId, cols, rows)` triples passed to [`MuxRouter::resize_pane`] in call order.
    #[must_use]
    pub fn resize_pane_calls(&self) -> Vec<(PaneId, u16, u16)> {
        self.calls.lock().resize_pane.clone()
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

    fn resize_pane(&self, id: PaneId, cols: u16, rows: u16) -> Result<(), MuxError> {
        self.calls.lock().resize_pane.push((id, cols, rows));
        self.delegate.resize_pane(id, cols, rows)
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
        let info = mux.tab_info(id).unwrap();
        assert_eq!(info.pane_ids.len(), 1);
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
    fn pane_geometry_matches_creation() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(120, 40);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        let pane = mux.pane_info(pane_id).unwrap();
        assert_eq!(
            pane.geometry,
            Geometry {
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
        // active is _b; also open a third
        let _c = mux.create_tab(80, 24);
        let closed_id = mux.active_tab_id().unwrap();
        mux.close_tab(closed_id).unwrap();
        let new_active = mux.active_tab_id().expect("another tab should be active");
        assert_ne!(new_active, closed_id);
        assert!(mux.tab_ids().contains(&new_active));
        // a and _b still present (only _c was closed)
        assert!(mux.tab_ids().contains(&a));
    }

    #[test]
    fn close_non_active_tab_preserves_active() {
        let mux = InMemoryMux::new();
        let a = mux.create_tab(80, 24);
        let b = mux.create_tab(80, 24);
        // active is b; close a
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
    fn resize_pane_updates_geometry() {
        let mux = InMemoryMux::new();
        let tab_id = mux.create_tab(80, 24);
        let pane_id = mux.tab_info(tab_id).unwrap().pane_ids[0];
        mux.resize_pane(pane_id, 132, 50).unwrap();
        assert_eq!(
            mux.pane_info(pane_id).unwrap().geometry,
            Geometry {
                cols: 132,
                rows: 50
            }
        );
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
            mux.resize_pane(pane_id, 80, 24),
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
        mock.resize_pane(pane_id, 132, 50).unwrap();
        assert_eq!(mock.resize_pane_calls(), vec![(pane_id, 132u16, 50u16)]);
    }

    #[test]
    fn mock_state_is_live_after_calls() {
        let mock = MockMuxRouter::new();
        let tab_id = mock.create_tab(80, 24);
        assert_eq!(mock.active_tab_id(), Some(tab_id));
        mock.close_tab(tab_id).unwrap();
        assert_eq!(mock.active_tab_id(), None);
    }
}
