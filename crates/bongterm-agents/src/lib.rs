#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod approval;
pub mod classify;
pub mod claude_code;
pub mod codex_cli;
pub mod corpus;
pub mod discover;
pub mod file_change;
pub mod lifecycle;
pub mod replay;
pub mod transcript;

/// How well `BongTerm` can observe and interact with the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityLevel {
    Full,
    Partial,
    None,
}

/// Reliability grade of an adapter's classification accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reliability {
    High,
    Medium,
    Low,
}

/// How MCP is supported by this agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpSupport {
    /// Agent supports MCP natively via JSON config.
    Native,
    /// Partial support (e.g., env-var injection only).
    Partial,
    /// Not supported.
    None,
}

/// Whether mid-session steering (interrupt/inject) is supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlChannel {
    /// Agent exposes supported IPC/API for steering.
    Supported,
    /// Steering is not supported — mark unavailable in UI, never simulate.
    Unavailable,
}

/// How the agent was detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectionMode {
    BinaryOnPath,
    ExplicitConfig,
    AutoDiscover,
}

/// How the agent should be launched.
#[derive(Debug, Clone)]
pub enum LaunchMode {
    /// Run as a subprocess, capturing stdout/stderr via PTY.
    Subprocess,
    /// Run via a specific wrapper (e.g., WSL).
    Wrapped { wrapper: String },
}

/// Capabilities reported by an adapter after discovery.
#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    pub name: String,
    pub version: Option<String>,
    pub capability_level: CapabilityLevel,
    pub reliability: Reliability,
    pub mcp_support: McpSupport,
    pub control_channel: ControlChannel,
    pub detection_mode: DetectionMode,
    pub launch_mode: LaunchMode,
}

/// Spec for launching an agent subprocess.
#[derive(Debug, Clone)]
pub struct AgentLaunchSpec {
    pub binary: String,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
}

/// Spec for the OS process.
#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub launch: AgentLaunchSpec,
    pub rss_limit_bytes: u64,
    pub cpu_rate_bps: u32,
}

/// A chunk of output from the agent subprocess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputChunk {
    pub bytes: Vec<u8>,
    pub from_stderr: bool,
}

/// A classified event from the agent output stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    /// Raw output chunk.
    Output(OutputChunk),
    /// Agent issued a tool call.
    ToolCall { tool_name: String, raw_json: String },
    /// Tool call completed.
    ToolResult { tool_name: String, success: bool },
    /// Agent produced a final response or stopped.
    Completed { exit_code: i32 },
    /// Classifier detected a possible prompt injection attempt.
    SuspectedInjection { excerpt: String },
}

/// How the agent subprocess exited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitState {
    Clean { exit_code: i32 },
    Crashed { signal: Option<i32> },
    TimedOut,
    Killed,
}

/// Summary produced after an agent run completes.
#[derive(Debug, Clone)]
pub struct AgentExitSummary {
    pub exit_state: ExitState,
    pub tool_calls_made: u64,
    pub output_bytes: u64,
    /// Short text suitable for replay pre-fill.
    pub replay_summary: Option<String>,
}

/// Authentication state of the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    Authenticated,
    Unauthenticated,
    Expired,
    Unknown,
}

/// Result of attempting to discover an agent binary.
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub found: bool,
    pub binary_path: Option<String>,
    pub version: Option<String>,
    pub auth_state: AuthState,
}

/// Error type for agent adapter operations.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("agent binary not found: {0}")]
    NotFound(String),
    #[error("agent not authenticated: {0}")]
    NotAuthenticated(String),
    #[error("launch error: {0}")]
    Launch(String),
    #[error("classifier error: {0}")]
    Classifier(String),
}

/// Classifies the output stream of a running agent process.
pub trait AgentOutputClassifier: Send {
    /// Ingest raw bytes and return a receiver for classified events.
    /// The receiver is created when the classifier is constructed.
    fn event_receiver(&mut self) -> tokio::sync::mpsc::Receiver<AgentEvent>;

    /// Feed raw bytes into the classifier.
    fn ingest(&mut self, chunk: &OutputChunk);

    /// Signal that the process has exited; produce final summary.
    fn finalize(&mut self, exit_state: ExitState) -> AgentExitSummary;
}

/// Port interface for a CLI coding agent (e.g., Claude Code, Codex CLI).
pub trait AgentAdapter: Send + Sync {
    /// Discover the agent binary and auth state.
    fn discover(&self) -> DiscoveryResult;

    /// Return the capabilities of this adapter.
    fn capabilities(&self) -> AgentCapabilities;

    /// Build the process spec for a new agent run.
    fn build_process_spec(&self, cwd: &str, prompt: &str) -> Result<ProcessSpec, AgentError>;

    /// Create a new output classifier for a run.
    fn create_classifier(&self) -> Box<dyn AgentOutputClassifier>;

    /// Produce a post-run summary suitable for replay pre-fill.
    /// `tool_calls_made` and `output_bytes` are tallied by the supervisor.
    fn summarize_exit(
        &self,
        exit_state: ExitState,
        tool_calls_made: u64,
        output_bytes: u64,
    ) -> AgentExitSummary {
        AgentExitSummary {
            exit_state,
            tool_calls_made,
            output_bytes,
            replay_summary: Some(format!(
                "Re-run {} ({} tool calls, {} bytes)",
                self.capabilities().name,
                tool_calls_made,
                output_bytes
            )),
        }
    }
}

use std::sync::{Arc, Mutex};

/// Mock classifier for testing: injects preset events then echoes raw output.
pub struct MockAgentOutputClassifier {
    events_to_inject: Vec<AgentEvent>,
    tx: tokio::sync::mpsc::Sender<AgentEvent>,
    rx: Option<tokio::sync::mpsc::Receiver<AgentEvent>>,
}

impl MockAgentOutputClassifier {
    /// Create a new mock classifier that will emit `events_to_inject` on first ingest.
    #[must_use]
    pub fn new(events_to_inject: Vec<AgentEvent>) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        Self {
            events_to_inject,
            tx,
            rx: Some(rx),
        }
    }
}

impl AgentOutputClassifier for MockAgentOutputClassifier {
    /// # Panics
    ///
    /// Panics if called more than once.
    fn event_receiver(&mut self) -> tokio::sync::mpsc::Receiver<AgentEvent> {
        self.rx.take().expect("event_receiver called twice")
    }

    fn ingest(&mut self, chunk: &OutputChunk) {
        // Emit injected events (pre-set) then echo raw output.
        for event in self.events_to_inject.drain(..) {
            let _ = self.tx.try_send(event);
        }
        let _ = self.tx.try_send(AgentEvent::Output(chunk.clone()));
    }

    fn finalize(&mut self, exit_state: ExitState) -> AgentExitSummary {
        AgentExitSummary {
            exit_state,
            tool_calls_made: 0,
            output_bytes: 0,
            replay_summary: None,
        }
    }
}

/// Mock adapter for testing: returns preset events from each classifier instance.
pub struct MockAgentAdapter {
    preset_events: Arc<Mutex<Vec<AgentEvent>>>,
    name: String,
}

impl MockAgentAdapter {
    /// Create a new mock adapter with the given name and preset events.
    #[must_use]
    pub fn new(name: impl Into<String>, events: Vec<AgentEvent>) -> Self {
        Self {
            preset_events: Arc::new(Mutex::new(events)),
            name: name.into(),
        }
    }
}

impl AgentAdapter for MockAgentAdapter {
    fn discover(&self) -> DiscoveryResult {
        DiscoveryResult {
            found: true,
            binary_path: Some(format!("/usr/local/bin/{}", self.name)),
            version: Some("0.0.0-mock".to_string()),
            auth_state: AuthState::Authenticated,
        }
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities {
            name: self.name.clone(),
            version: Some("0.0.0-mock".to_string()),
            capability_level: CapabilityLevel::Full,
            reliability: Reliability::High,
            mcp_support: McpSupport::Native,
            control_channel: ControlChannel::Unavailable,
            detection_mode: DetectionMode::BinaryOnPath,
            launch_mode: LaunchMode::Subprocess,
        }
    }

    fn build_process_spec(&self, cwd: &str, prompt: &str) -> Result<ProcessSpec, AgentError> {
        Ok(ProcessSpec {
            launch: AgentLaunchSpec {
                binary: format!("/usr/local/bin/{}", self.name),
                argv: vec![prompt.to_string()],
                env: Vec::new(),
                cwd: Some(cwd.to_string()),
            },
            rss_limit_bytes: 512 * 1024 * 1024,
            cpu_rate_bps: 8000,
        })
    }

    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    fn create_classifier(&self) -> Box<dyn AgentOutputClassifier> {
        let events = self.preset_events.lock().unwrap().clone();
        Box::new(MockAgentOutputClassifier::new(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_classifier_emits_injected_events() {
        let events = vec![AgentEvent::ToolCall {
            tool_name: "read_file".to_string(),
            raw_json: "{}".to_string(),
        }];
        let mut classifier = MockAgentOutputClassifier::new(events.clone());
        let mut rx = classifier.event_receiver();

        let chunk = OutputChunk {
            bytes: b"some output".to_vec(),
            from_stderr: false,
        };
        classifier.ingest(&chunk);

        // Should receive the injected tool-call event first.
        let event = rx.try_recv().unwrap();
        assert_eq!(
            event,
            AgentEvent::ToolCall {
                tool_name: "read_file".to_string(),
                raw_json: "{}".to_string()
            }
        );
    }

    #[test]
    fn mock_adapter_create_classifier_returns_preset_events() {
        let events = vec![AgentEvent::Completed { exit_code: 0 }];
        let adapter = MockAgentAdapter::new("mock-agent", events.clone());
        let mut classifier = adapter.create_classifier();
        let mut rx = classifier.event_receiver();

        let chunk = OutputChunk {
            bytes: b"done".to_vec(),
            from_stderr: false,
        };
        classifier.ingest(&chunk);

        let event = rx.try_recv().unwrap();
        assert_eq!(event, AgentEvent::Completed { exit_code: 0 });
    }

    #[test]
    fn mock_adapter_discovery_returns_found() {
        let adapter = MockAgentAdapter::new("claude-code", Vec::new());
        let result = adapter.discover();
        assert!(result.found);
        assert_eq!(result.auth_state, AuthState::Authenticated);
    }

    #[test]
    fn mock_adapter_summarize_exit_produces_replay_summary() {
        let adapter = MockAgentAdapter::new("claude-code", Vec::new());
        let summary = adapter.summarize_exit(ExitState::Clean { exit_code: 0 }, 3, 1024);
        assert_eq!(summary.tool_calls_made, 3);
        assert_eq!(summary.output_bytes, 1024);
        assert!(
            summary.replay_summary.is_some(),
            "summarize_exit must populate replay_summary for replay pre-fill"
        );
    }
}
