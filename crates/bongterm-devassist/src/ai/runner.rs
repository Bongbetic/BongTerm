//! AI backend port + Claude Code subprocess runner.
//!
//! The backend is PREVIEW-ONLY: it returns suggestions, never executes them.
//! Execution is gated behind `super::cmdk::CmdKSession::confirm_run`.

use crate::DevassistError;

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
}
