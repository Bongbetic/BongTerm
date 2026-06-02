//! AI backend port + Claude Code subprocess runner.
//!
//! The backend is PREVIEW-ONLY: it returns suggestions, never executes them.
//! Execution is gated behind `super::cmdk::CmdKSession::confirm_run`.

use crate::DevassistError;
use std::process::Command;

/// What the caller wants the AI to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiIntent {
    /// Natural-language to shell command (Cmd-K).
    NlToCommand,
    /// Explain a failed command.
    ExplainFailure,
}

/// Read-only context handed to the backend. Never contains secrets.
#[derive(Debug, Clone)]
pub struct AiContext {
    pub cwd: String,
    pub shell: String,
    /// The failed command text, when intent is `ExplainFailure`.
    pub failed_command: Option<String>,
    /// Redacted tail of recent output; bounded length.
    pub transcript_tail: String,
}

/// A single AI request.
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub intent: AiIntent,
    pub user_text: String,
    pub context: AiContext,
}

/// A preview-only suggestion. `command` is never auto-run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSuggestion {
    /// The suggested command text (preview only).
    pub command: String,
    /// Human-readable rationale or explanation.
    pub explanation: String,
}

/// Whether the AI backend can be used right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiAvailability {
    Available { version: String },
    Unavailable { reason: String },
}

/// Port for an AI backend. Implementations run off the terminal hot path.
pub trait AiBackend: Send + Sync {
    /// Report whether the backend is usable, for example Claude Code installed.
    fn availability(&self) -> AiAvailability;

    /// Produce a preview-only suggestion. Must not execute anything.
    fn suggest(&self, request: &AiRequest) -> Result<AiSuggestion, DevassistError>;
}

/// Backend used when Claude Code is unavailable.
#[derive(Debug, Clone)]
pub struct UnavailableBackend {
    reason: String,
}

impl UnavailableBackend {
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl AiBackend for UnavailableBackend {
    fn availability(&self) -> AiAvailability {
        AiAvailability::Unavailable {
            reason: self.reason.clone(),
        }
    }

    fn suggest(&self, _request: &AiRequest) -> Result<AiSuggestion, DevassistError> {
        Err(DevassistError::Unavailable(self.reason.clone()))
    }
}

/// Discovered Claude Code binary and version.
#[derive(Debug, Clone)]
pub struct ClaudeInfo {
    pub binary: String,
    pub version: String,
}

/// Port that locates the Claude Code CLI.
pub trait ClaudeProbe: Send + Sync {
    fn locate(&self) -> Option<ClaudeInfo>;
}

/// Build the appropriate backend from a probe result.
#[must_use]
pub fn detect_backend(probe: &dyn ClaudeProbe) -> Box<dyn AiBackend> {
    match probe.locate() {
        Some(info) => Box::new(ClaudeCodeAiRunner::new(info)),
        None => Box::new(UnavailableBackend::new(
            "Claude Code not installed. Install the Claude Code CLI to enable Cmd-K and the failed-command explainer.",
        )),
    }
}

/// Wraps the Claude Code CLI in non-interactive mode.
#[derive(Debug, Clone)]
pub struct ClaudeCodeAiRunner {
    info: ClaudeInfo,
}

impl ClaudeCodeAiRunner {
    #[must_use]
    pub fn new(info: ClaudeInfo) -> Self {
        Self { info }
    }

    /// Discover a local `claude` binary and read its version.
    pub fn discover() -> Result<Self, DevassistError> {
        let output = Command::new("claude")
            .arg("--version")
            .output()
            .map_err(|error| {
                DevassistError::Backend(format!("unable to run `claude --version`: {error}"))
            })?;
        if !output.status.success() {
            return Err(DevassistError::Unavailable(
                "Claude Code not installed".to_string(),
            ));
        }
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Self::new(ClaudeInfo {
            binary: "claude".to_string(),
            version,
        }))
    }

    /// Parse `claude --print --output-format json` stdout.
    pub fn parse_print_json(
        stdout: &str,
        intent: AiIntent,
    ) -> Result<AiSuggestion, DevassistError> {
        let value: serde_json::Value = serde_json::from_str(stdout.trim())
            .map_err(|e| DevassistError::Parse(format!("claude json: {e}")))?;
        let result = value
            .get("result")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| DevassistError::Parse("missing `result` field".to_string()))?
            .to_string();

        Ok(match intent {
            AiIntent::NlToCommand => AiSuggestion {
                command: result,
                explanation: String::new(),
            },
            AiIntent::ExplainFailure => AiSuggestion {
                command: String::new(),
                explanation: result,
            },
        })
    }

    /// Build argv for a non-interactive Claude Code invocation.
    #[must_use]
    pub fn build_argv(&self, prompt: &str) -> Vec<String> {
        vec![
            self.info.binary.clone(),
            "--print".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
            "--prompt".to_string(),
            prompt.to_string(),
        ]
    }
}

impl AiBackend for ClaudeCodeAiRunner {
    fn availability(&self) -> AiAvailability {
        AiAvailability::Available {
            version: self.info.version.clone(),
        }
    }

    fn suggest(&self, request: &AiRequest) -> Result<AiSuggestion, DevassistError> {
        let prompt = format!(
            "{}\ncwd: {}\nshell: {}\n{}",
            request.user_text,
            request.context.cwd,
            request.context.shell,
            request.context.transcript_tail
        );
        let argv = self.build_argv(&prompt);
        let output = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .output()
            .map_err(|e| DevassistError::Backend(format!("spawn claude: {e}")))?;
        if !output.status.success() {
            return Err(DevassistError::Backend(format!(
                "claude exited with {}",
                output.status
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_print_json(&stdout, request.intent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_backend_reports_unavailable() {
        let backend = UnavailableBackend::new("Claude Code not installed");
        match backend.availability() {
            AiAvailability::Unavailable { reason } => {
                assert!(reason.contains("not installed"));
            }
            AiAvailability::Available { .. } => panic!("expected unavailable"),
        }
    }

    #[test]
    fn request_carries_context_and_intent() {
        let req = AiRequest {
            intent: AiIntent::NlToCommand,
            user_text: "list files sorted by size".to_string(),
            context: AiContext {
                cwd: "C:\\proj".to_string(),
                shell: "pwsh".to_string(),
                failed_command: None,
                transcript_tail: String::new(),
            },
        };
        assert_eq!(req.intent, AiIntent::NlToCommand);
        assert!(req.user_text.contains("size"));
    }

    struct FakeProbeFound;

    impl ClaudeProbe for FakeProbeFound {
        fn locate(&self) -> Option<ClaudeInfo> {
            Some(ClaudeInfo {
                binary: "claude".to_string(),
                version: "1.2.3".to_string(),
            })
        }
    }

    struct FakeProbeMissing;

    impl ClaudeProbe for FakeProbeMissing {
        fn locate(&self) -> Option<ClaudeInfo> {
            None
        }
    }

    #[test]
    fn detect_backend_available_when_probe_finds_claude() {
        let backend = detect_backend(&FakeProbeFound);
        assert!(matches!(
            backend.availability(),
            AiAvailability::Available { version } if version == "1.2.3"
        ));
    }

    #[test]
    fn detect_backend_unavailable_when_claude_missing() {
        let backend = detect_backend(&FakeProbeMissing);
        match backend.availability() {
            AiAvailability::Unavailable { reason } => {
                assert!(reason.to_lowercase().contains("claude code not installed"));
            }
            AiAvailability::Available { .. } => panic!("expected unavailable"),
        }
    }

    #[test]
    fn claude_runner_parses_json_print_output() {
        let stdout = r#"{"type":"result","result":"Get-ChildItem | Sort-Object Length"}"#;
        let suggestion = ClaudeCodeAiRunner::parse_print_json(stdout, AiIntent::NlToCommand)
            .expect("parse should succeed");
        assert_eq!(suggestion.command, "Get-ChildItem | Sort-Object Length");
    }

    #[test]
    fn claude_runner_explain_intent_puts_text_in_explanation() {
        let stdout =
            r#"{"type":"result","result":"exit 127 means the command was not found in PATH"}"#;
        let suggestion =
            ClaudeCodeAiRunner::parse_print_json(stdout, AiIntent::ExplainFailure).unwrap();
        assert!(suggestion.explanation.contains("not found in PATH"));
        assert!(suggestion.command.is_empty());
    }

    #[test]
    fn claude_runner_rejects_malformed_json() {
        let err =
            ClaudeCodeAiRunner::parse_print_json("not json", AiIntent::NlToCommand).unwrap_err();
        assert!(matches!(err, DevassistError::Parse(_)));
    }
}
