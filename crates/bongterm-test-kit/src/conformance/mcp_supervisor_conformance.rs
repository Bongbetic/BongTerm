//! Conformance suite for `bongterm_mcp::supervisor::Supervisor`.

use bongterm_mcp::supervisor::{IdleShutdownOutcome, Supervisor, WorkspaceId};
use bongterm_mcp::{McpServerConfig, MockMcpTransport};

/// Exercise one-process-per-server + idle-shutdown invariants.
///
/// # Panics
/// Panics if any conformance assertion fails.
pub fn run() {
    let sup = Supervisor::new();
    let ws = WorkspaceId("conformance-ws".to_string());
    let cfg = McpServerConfig {
        name: "srv".to_string(),
        argv: vec!["node".to_string()],
        env: vec![],
    };

    sup.register(ws.clone(), cfg.clone(), Box::new(MockMcpTransport::new()))
        .expect("first register must succeed");
    assert!(
        sup.register(ws.clone(), cfg, Box::new(MockMcpTransport::new()))
            .is_err(),
        "duplicate (workspace, server) must be rejected"
    );
    assert_eq!(sup.server_count(&ws), 1);
    assert_eq!(
        sup.try_idle_shutdown(&ws, "srv"),
        IdleShutdownOutcome::Stopped,
        "idle shutdown with no attached agent must stop server"
    );
    assert_eq!(sup.server_count(&ws), 0);
}

#[cfg(test)]
mod tests {
    #[test]
    fn supervisor_conformance_passes_for_real_supervisor() {
        super::run();
    }
}
