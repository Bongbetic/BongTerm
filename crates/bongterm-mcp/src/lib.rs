//! bongterm-mcp — MCP transport port interface.
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

/// Configuration for starting an MCP server process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    /// Full argv for the server process. `argv[0]` is the executable.
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// A single MCP tool descriptor returned by `list_tools`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    /// JSON schema as a string (simplified for scaffold).
    pub input_schema_json: String,
}

/// A tool call request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolRequest {
    pub tool_name: String,
    /// Arguments as a JSON value.
    pub arguments_json: String,
}

/// A tool call response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolResponse {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpContent {
    pub content_type: String,
    pub text: String,
}

/// Reason a transport was stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    UserRequested,
    IdleTimeout,
    HealthCheckFailed,
    Error(String),
}

/// Metrics from an MCP transport.
#[derive(Debug, Clone, Default)]
pub struct McpMetrics {
    pub calls_total: u64,
    pub calls_failed: u64,
    pub rss_bytes: u64,
}

/// Port interface for a single MCP server transport.
/// One transport per server process. Implementations in `bongterm-mcp` (production),
/// `MockMcpTransport` (tests).
pub trait McpTransport: Send + Sync {
    /// Start the MCP server process using the given config.
    /// MUST reject configs where argv contains "npx" followed by "-y" (spec §3.4).
    fn start(&self, config: McpServerConfig) -> Result<(), McpError>;

    /// List available tools (requires server to be started).
    fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError>;

    /// Call a tool and return the response.
    fn call_tool(&self, request: McpToolRequest) -> Result<McpToolResponse, McpError>;

    /// Stop the server process.
    fn stop(&self, reason: StopReason) -> Result<(), McpError>;

    /// Collect current metrics.
    fn collect_metrics(&self) -> McpMetrics;
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("forbidden argv: {0}")]
    ForbiddenArgv(String),
    #[error("server not started")]
    NotStarted,
    #[error("server already started")]
    AlreadyStarted,
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("transport error: {0}")]
    Transport(String),
}

use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct MockState {
    started: bool,
    tools: Vec<McpToolDescriptor>,
    calls: u64,
}

pub struct MockMcpTransport {
    state: Arc<Mutex<MockState>>,
    /// Tools to return from `list_tools` (set during construction).
    preset_tools: Vec<McpToolDescriptor>,
}

impl MockMcpTransport {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
            preset_tools: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_tools(tools: Vec<McpToolDescriptor>) -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
            preset_tools: tools,
        }
    }

    fn check_forbidden_argv(argv: &[String]) -> Result<(), McpError> {
        for window in argv.windows(2) {
            if window[0] == "npx" && window[1] == "-y" {
                return Err(McpError::ForbiddenArgv(
                    "npx -y is forbidden: auto-install not allowed (spec §3.4)".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Default for MockMcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl McpTransport for MockMcpTransport {
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    fn start(&self, config: McpServerConfig) -> Result<(), McpError> {
        Self::check_forbidden_argv(&config.argv)?;
        let mut s = self.state.lock().unwrap();
        if s.started {
            return Err(McpError::AlreadyStarted);
        }
        s.started = true;
        s.tools.clone_from(&self.preset_tools);
        Ok(())
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        let s = self.state.lock().unwrap();
        if !s.started {
            return Err(McpError::NotStarted);
        }
        Ok(s.tools.clone())
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    fn call_tool(&self, _request: McpToolRequest) -> Result<McpToolResponse, McpError> {
        let mut s = self.state.lock().unwrap();
        if !s.started {
            return Err(McpError::NotStarted);
        }
        s.calls += 1;
        Ok(McpToolResponse {
            content: vec![McpContent {
                content_type: "text".to_string(),
                text: "mock response".to_string(),
            }],
            is_error: false,
        })
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    fn stop(&self, _reason: StopReason) -> Result<(), McpError> {
        self.state.lock().unwrap().started = false;
        Ok(())
    }

    /// # Panics
    /// Panics if the internal mutex is poisoned.
    fn collect_metrics(&self) -> McpMetrics {
        let s = self.state.lock().unwrap();
        McpMetrics {
            calls_total: s.calls,
            calls_failed: 0,
            rss_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_npx_minus_y() {
        let mock = MockMcpTransport::new();
        let config = McpServerConfig {
            name: "test-server".to_string(),
            argv: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-test".to_string(),
            ],
            env: vec![],
        };
        let err = mock.start(config).unwrap_err();
        assert!(
            matches!(err, McpError::ForbiddenArgv(_)),
            "expected ForbiddenArgv, got {err:?}"
        );
    }

    #[test]
    fn allows_non_npx_y_argv() {
        let mock = MockMcpTransport::new();
        let config = McpServerConfig {
            name: "safe-server".to_string(),
            argv: vec!["node".to_string(), "server.js".to_string()],
            env: vec![],
        };
        assert!(mock.start(config).is_ok());
    }

    #[test]
    fn allows_npx_without_y() {
        let mock = MockMcpTransport::new();
        let config = McpServerConfig {
            name: "npx-server".to_string(),
            argv: vec![
                "npx".to_string(),
                "@modelcontextprotocol/server-test".to_string(),
            ],
            env: vec![],
        };
        assert!(mock.start(config).is_ok());
    }

    #[test]
    fn list_tools_fails_when_not_started() {
        let mock = MockMcpTransport::new();
        assert!(matches!(
            mock.list_tools().unwrap_err(),
            McpError::NotStarted
        ));
    }

    #[test]
    fn call_tool_records_count() {
        let mock = MockMcpTransport::new();
        let config = McpServerConfig {
            name: "s".to_string(),
            argv: vec!["node".to_string()],
            env: vec![],
        };
        mock.start(config).unwrap();
        let req = McpToolRequest {
            tool_name: "t".to_string(),
            arguments_json: "{}".to_string(),
        };
        mock.call_tool(req).unwrap();
        assert_eq!(mock.collect_metrics().calls_total, 1);
    }
}
