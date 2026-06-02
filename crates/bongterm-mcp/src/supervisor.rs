//! MCP server supervision: one process per server per workspace registry.
//!
//! Spec §3.4. This crate owns governance and lifecycle coordination for MCP
//! server registration.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use bongterm_process_control::{JobObjectCaps, ProcessGovernor, ProcessHandle};

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

/// Identifies an attached agent session keeping an MCP server alive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentSessionId(pub String);

/// Result of attempting an idle shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleShutdownOutcome {
    /// Server had no attached agents and was stopped.
    Stopped,
    /// Shutdown refused: at least one active agent is attached.
    BlockedActiveAgent,
}

/// Health state of a supervised MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerHealth {
    Healthy,
    /// Three failures inside the window — auto-restart disabled until user re-enables.
    Unhealthy,
}

/// What the supervisor should do after a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartAction {
    RetryAfter(Duration),
    MarkUnhealthy,
}

/// Restart backoff with failure ceiling.
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    schedule: Vec<Duration>,
    failures: usize,
    health: ServerHealth,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            schedule: vec![
                Duration::from_secs(1),
                Duration::from_secs(5),
                Duration::from_secs(30),
            ],
            failures: 0,
            health: ServerHealth::Healthy,
        }
    }
}

impl RestartPolicy {
    pub fn record_failure(&mut self) -> RestartAction {
        if self.failures < self.schedule.len() {
            let action = RestartAction::RetryAfter(self.schedule[self.failures]);
            self.failures += 1;
            action
        } else {
            self.health = ServerHealth::Unhealthy;
            RestartAction::MarkUnhealthy
        }
    }

    pub fn record_success(&mut self) {
        self.failures = 0;
        self.health = ServerHealth::Healthy;
    }

    #[must_use]
    pub fn health(&self) -> ServerHealth {
        self.health
    }
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
    caps: Option<JobObjectCaps>,
    agents: HashSet<AgentSessionId>,
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
                caps: None,
                agents: HashSet::new(),
            },
        );
        Ok(())
    }

    /// Register a server and attach `JobObject` caps to its process handle.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn register_with_caps(
        &self,
        workspace: WorkspaceId,
        config: McpServerConfig,
        transport: Box<dyn McpTransport>,
        caps: JobObjectCaps,
        handle: ProcessHandle,
        governor: &dyn ProcessGovernor,
    ) -> Result<(), SupervisorError> {
        let key = ServerKey {
            workspace,
            server_name: config.name.clone(),
        };
        let mut map = self.entries.lock().unwrap();
        if map.contains_key(&key) {
            return Err(SupervisorError::AlreadyRegistered(key.server_name));
        }
        governor
            .attach(handle, caps)
            .map_err(|e| SupervisorError::Transport(McpError::Transport(e.to_string())))?;
        map.insert(
            key,
            Entry {
                config,
                transport,
                caps: Some(caps),
                agents: HashSet::new(),
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

    /// Return the caps attached to a server, if any.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn caps_for(&self, workspace: &WorkspaceId, server_name: &str) -> Option<JobObjectCaps> {
        let key = ServerKey {
            workspace: workspace.clone(),
            server_name: server_name.to_string(),
        };
        self.entries.lock().unwrap().get(&key).and_then(|e| e.caps)
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn attach_agent(
        &self,
        workspace: &WorkspaceId,
        server: &str,
        agent: AgentSessionId,
    ) -> Result<(), SupervisorError> {
        let key = ServerKey {
            workspace: workspace.clone(),
            server_name: server.to_string(),
        };
        let mut map = self.entries.lock().unwrap();
        let entry = map
            .get_mut(&key)
            .ok_or_else(|| SupervisorError::NotRegistered(server.to_string()))?;
        entry.agents.insert(agent);
        Ok(())
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn detach_agent(
        &self,
        workspace: &WorkspaceId,
        server: &str,
        agent: &AgentSessionId,
    ) -> Result<(), SupervisorError> {
        let key = ServerKey {
            workspace: workspace.clone(),
            server_name: server.to_string(),
        };
        let mut map = self.entries.lock().unwrap();
        let entry = map
            .get_mut(&key)
            .ok_or_else(|| SupervisorError::NotRegistered(server.to_string()))?;
        entry.agents.remove(agent);
        Ok(())
    }

    /// Stop server only if no agent session is attached.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn try_idle_shutdown(&self, workspace: &WorkspaceId, server: &str) -> IdleShutdownOutcome {
        let key = ServerKey {
            workspace: workspace.clone(),
            server_name: server.to_string(),
        };
        let mut map = self.entries.lock().unwrap();
        let Some(entry) = map.get(&key) else {
            return IdleShutdownOutcome::Stopped;
        };
        if !entry.agents.is_empty() {
            return IdleShutdownOutcome::BlockedActiveAgent;
        }
        let _ = entry.transport.stop(crate::StopReason::IdleTimeout);
        map.remove(&key);
        IdleShutdownOutcome::Stopped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockMcpTransport;
    use bongterm_process_control::{JobObjectCaps, MockProcessGovernor, ProcessHandle};

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

    #[test]
    fn registers_server_under_job_object_caps() {
        let sup = Supervisor::new();
        let ws = WorkspaceId("ws-1".into());
        let caps = JobObjectCaps {
            rss_bytes: 60 * 1024 * 1024,
            cpu_rate_bps: 5000,
            child_proc_count: 4,
        };
        let gov = MockProcessGovernor::new();
        sup.register_with_caps(
            ws.clone(),
            mk_cfg("fs"),
            Box::new(MockMcpTransport::new()),
            caps,
            ProcessHandle(4321),
            &gov,
        )
        .unwrap();
        assert_eq!(
            gov.caps_for(ProcessHandle(4321)),
            Some(caps),
            "caps must be attached at registration"
        );
        assert_eq!(sup.caps_for(&ws, "fs"), Some(caps));
    }

    #[test]
    fn idle_shutdown_blocked_while_agent_attached() {
        let sup = Supervisor::new();
        let ws = WorkspaceId("ws-1".into());
        sup.register(ws.clone(), mk_cfg("fs"), Box::new(MockMcpTransport::new()))
            .unwrap();
        sup.attach_agent(&ws, "fs", AgentSessionId("a1".into()))
            .unwrap();
        assert_eq!(
            sup.try_idle_shutdown(&ws, "fs"),
            IdleShutdownOutcome::BlockedActiveAgent
        );
        assert_eq!(sup.server_count(&ws), 1);
        sup.detach_agent(&ws, "fs", &AgentSessionId("a1".into()))
            .unwrap();
        assert_eq!(
            sup.try_idle_shutdown(&ws, "fs"),
            IdleShutdownOutcome::Stopped
        );
        assert_eq!(sup.server_count(&ws), 0);
    }

    #[test]
    fn restart_backoff_escalates_then_marks_unhealthy() {
        let mut policy = RestartPolicy::default();
        assert_eq!(
            policy.record_failure(),
            RestartAction::RetryAfter(Duration::from_secs(1))
        );
        assert_eq!(
            policy.record_failure(),
            RestartAction::RetryAfter(Duration::from_secs(5))
        );
        assert_eq!(
            policy.record_failure(),
            RestartAction::RetryAfter(Duration::from_secs(30))
        );
        assert_eq!(policy.record_failure(), RestartAction::MarkUnhealthy);
        assert_eq!(policy.health(), ServerHealth::Unhealthy);
        policy.record_success();
        assert_eq!(policy.health(), ServerHealth::Healthy);
        assert_eq!(
            policy.record_failure(),
            RestartAction::RetryAfter(Duration::from_secs(1))
        );
    }
}
