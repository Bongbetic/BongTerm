//! Conformance suite for [`bongterm_mcp::McpTransport`].

use bongterm_mcp::{McpServerConfig, McpTransport};

/// Run happy-path conformance checks against any [`McpTransport`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(transport: &impl McpTransport) {
    let config = McpServerConfig {
        name: "conformance-test-server".to_string(),
        argv: vec!["node".to_string(), "server.js".to_string()],
        env: Vec::new(),
    };

    assert!(
        transport.start(config).is_ok(),
        "start() with a valid (non-npx-y) config must return Ok"
    );

    assert!(
        transport.list_tools().is_ok(),
        "list_tools() must return Ok after a successful start"
    );
}
