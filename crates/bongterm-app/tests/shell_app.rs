//! Runtime-shell composition tests.
//!
//! Phase 1 corrective task `1.R.1`: the app binary must boot the PRD shell
//! chrome from `bongterm-ui` around the existing live terminal runtime, not the
//! temporary one-pane terminal app directly.

#[test]
fn composed_app_boots_shell_chrome_and_terminal_runtime() {
    let (app, task) = bongterm_app::BongTermApp::boot();
    drop(task);

    assert_eq!(app.title(), "BongTerm - workspace");
    assert!(app.shell_region_names().contains(&"tab-strip"));
    assert!(app.shell_region_names().contains(&"agent-sidebar"));
    assert!(app.shell_region_names().contains(&"terminal-surface"));
    assert_eq!(app.terminal_grid_size(), (80, 24));
}

#[test]
fn composed_app_exposes_live_panel_view_models() {
    let (app, task) = bongterm_app::BongTermApp::boot();
    drop(task);

    let agents = app.agent_sidebar_snapshot();
    assert!(agents.agents.is_empty());
    assert!(agents.approvals.is_empty());

    let resources = app.resource_dashboard_snapshot();
    assert!(!resources.total_rss.is_empty());
    assert!(resources.rows.iter().any(|row| row.category == "BongTerm"));
}

#[test]
fn composed_app_resizes_terminal_from_center_pane_bounds() {
    let (mut app, task) = bongterm_app::BongTermApp::boot();
    drop(task);

    let surface = bongterm_app::BongTermApp::terminal_surface_size_for_window(1200.0, 720.0);
    assert_eq!(surface.width, 720.0);
    assert_eq!(surface.height, 604.0);

    let (cell_w, cell_h) = bongterm_render::startup_monospace_cell_size(14.0);
    let expected =
        bongterm_render::grid_dims(surface.width - 16.0, surface.height - 16.0, cell_w, cell_h);

    let task = app.update(bongterm_app::AppMessage::WindowResized(1200.0, 720.0));
    drop(task);

    assert_eq!(app.terminal_grid_size(), expected);
    assert_ne!(app.terminal_grid_size(), (140, 40));
}
