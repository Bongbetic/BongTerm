//! MCP server supervision: one process per server per workspace registry.
//!
//! Spec §3.4. This crate owns governance and lifecycle coordination for MCP
//! server registration.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::{McpError, McpServerConfig, McpTransport};

/// Identifies a workspace; MCP processes are scoped one-per-server-per-workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(pub String);

/// Composite key: a server name is unique within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerKey {
    pub workspace: WorkspaceId,
    pub server_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("server already registered: {0}")]
    AlreadyRegistered(String),
    #[error("server not registered: {0}")]
    NotRegistered(String),
    #[error("transport error: {0}")]
    Transport(#[from] McpError),
}

#[allow(dead_code)]
struct Entry {
    config: McpServerConfig,
    transport: Box<dyn McpTransport>,
}

/// The MCP supervisor registry. Holds at most one transport per `ServerKey`.
#[derive(Default)]
pub struct Supervisor {
    entries: Mutex<HashMap<ServerKey, Entry>>,
}

impl Supervisor {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Register a server transport for a workspace.
    ///
    /// Rejects a duplicate (workspace, server-name) pair.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn register(
        &self,
        workspace: WorkspaceId,
        config: McpServerConfig,
        transport: Box<dyn McpTransport>,
    ) -> Result<(), SupervisorError> {
        let key = ServerKey {
            workspace,
            server_name: config.name.clone(),
        };
        let mut map = self.entries.lock().unwrap();
        if map.contains_key(&key) {
            return Err(SupervisorError::AlreadyRegistered(key.server_name));
        }
        map.insert(
            key,
            Entry {
                config,
                transport,
            },
        );
        Ok(())
    }

    /// Count registered servers in a workspace.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn server_count(&self, workspace: &WorkspaceId) -> usize {
        self.entries
            .lock()
            .unwrap()
            .keys()
            .filter(|k| &k.workspace == workspace)
            .count()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockMcpTransport;

    fn mk_cfg(name: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.to_string(),
            argv: vec!["node".to_string(), "s.js".to_string()],
            env: vec![],
        }
    }

    #[test]
    fn one_process_per_server_per_workspace() {
        let sup = Supervisor::new();
        let ws = WorkspaceId("ws-1".into());
        sup.register(ws.clone(), mk_cfg("fs"), Box::new(MockMcpTransport::new()))
            .unwrap();
        // Re-registering the same (workspace, server-name) must be rejected: exactly one process.
        let dup = sup.register(ws.clone(), mk_cfg("fs"), Box::new(MockMcpTransport::new()));
        assert!(
            matches!(dup, Err(SupervisorError::AlreadyRegistered(_))),
            "got {dup:?}"
        );
        assert_eq!(sup.server_count(&ws), 1);
        // Same server name in a different workspace is a distinct process.
        let ws2 = WorkspaceId("ws-2".into());
        sup.register(ws2.clone(), mk_cfg("fs"), Box::new(MockMcpTransport::new()))
            .unwrap();
        assert_eq!(sup.server_count(&ws2), 1);
    }
}
