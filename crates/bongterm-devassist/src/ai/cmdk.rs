//! Cmd-K session: natural-language command assist, preview-only.
//!
//! The session never spawns a process. `confirm_run` only releases the previewed
//! command string after explicit user confirmation.

use crate::DevassistError;
use crate::ai::runner::{AiBackend, AiContext, AiIntent, AiRequest, AiSuggestion};

/// Read-only projection of `CmdKSession` for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdKView {
    Idle,
    Previewed { command: String },
    Unavailable { reason: String },
}

/// Lifecycle state of a Cmd-K session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmdKState {
    /// No preview requested yet.
    Idle,
    /// Preview is shown; awaiting explicit confirmation.
    Previewed,
    /// User explicitly confirmed; command may be passed to normal execution.
    Confirmed,
    /// Backend unavailable, for example Claude Code not installed.
    Unavailable,
}

/// Errors from the Cmd-K session.
#[derive(Debug, thiserror::Error)]
pub enum CmdKError {
    #[error("AI assist unavailable: {0}")]
    Unavailable(String),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("nothing to run: no confirmed preview")]
    NothingToRun,
}

/// Preview-only Cmd-K session over an [`AiBackend`].
pub struct CmdKSession {
    backend: Box<dyn AiBackend>,
    state: CmdKState,
    last_suggestion: Option<AiSuggestion>,
    unavailable_reason: Option<String>,
}

impl CmdKSession {
    #[must_use]
    pub fn new(backend: Box<dyn AiBackend>) -> Self {
        Self {
            backend,
            state: CmdKState::Idle,
            last_suggestion: None,
            unavailable_reason: None,
        }
    }

    #[must_use]
    pub fn state(&self) -> CmdKState {
        self.state
    }

    /// Request a preview-only suggestion. Does not execute anything.
    pub fn request_preview(
        &mut self,
        user_text: impl Into<String>,
        context: AiContext,
    ) -> Result<AiSuggestion, CmdKError> {
        let request = AiRequest {
            intent: AiIntent::NlToCommand,
            user_text: user_text.into(),
            context,
        };

        match self.backend.suggest(&request) {
            Ok(suggestion) => {
                self.last_suggestion = Some(suggestion.clone());
                self.unavailable_reason = None;
                self.state = CmdKState::Previewed;
                Ok(suggestion)
            }
            Err(DevassistError::Unavailable(reason)) => {
                self.state = CmdKState::Unavailable;
                self.unavailable_reason = Some(reason.clone());
                self.last_suggestion = None;
                Err(CmdKError::Unavailable(reason))
            }
            Err(other) => {
                self.state = CmdKState::Idle;
                self.last_suggestion = None;
                self.unavailable_reason = None;
                Err(CmdKError::Backend(other.to_string()))
            }
        }
    }

    /// Explicit Run confirmation. Returns a command only after preview.
    pub fn confirm_run(&mut self) -> Result<String, CmdKError> {
        match (self.state, &self.last_suggestion) {
            (CmdKState::Previewed, Some(suggestion)) => {
                let command = suggestion.command.clone();
                self.state = CmdKState::Confirmed;
                Ok(command)
            }
            _ => Err(CmdKError::NothingToRun),
        }
    }

    /// Read-only snapshot for UI projection. No backend handle is carried through
    /// to prevent the UI from triggering execution.
    #[must_use]
    pub fn view(&self) -> CmdKView {
        match self.state {
            CmdKState::Idle => CmdKView::Idle,
            CmdKState::Previewed | CmdKState::Confirmed => match &self.last_suggestion {
                Some(suggestion) => CmdKView::Previewed {
                    command: suggestion.command.clone(),
                },
                None => CmdKView::Idle,
            },
            CmdKState::Unavailable => CmdKView::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "AI assist unavailable".to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DevassistError;
    use crate::ai::runner::{AiAvailability, AiBackend, AiContext, AiRequest, AiSuggestion};

    struct StubBackend {
        result: Result<AiSuggestion, DevassistError>,
    }

    impl StubBackend {
        fn new(result: Result<AiSuggestion, DevassistError>) -> Self {
            Self { result }
        }
    }

    impl AiBackend for StubBackend {
        fn availability(&self) -> AiAvailability {
            if self.result.is_ok() {
                AiAvailability::Available {
                    version: "stub".to_string(),
                }
            } else {
                AiAvailability::Unavailable {
                    reason: "stub unavailable".to_string(),
                }
            }
        }

        fn suggest(&self, _request: &AiRequest) -> Result<AiSuggestion, DevassistError> {
            match &self.result {
                Ok(suggestion) => Ok(suggestion.clone()),
                Err(DevassistError::Backend(message)) => {
                    Err(DevassistError::Backend(message.clone()))
                }
                Err(DevassistError::Unavailable(message)) => {
                    Err(DevassistError::Unavailable(message.clone()))
                }
                Err(DevassistError::Parse(message)) => Err(DevassistError::Parse(message.clone())),
                Err(DevassistError::MissingParam(message)) => {
                    Err(DevassistError::MissingParam(message.clone()))
                }
                Err(DevassistError::Storage(message)) => {
                    Err(DevassistError::Storage(message.clone()))
                }
                Err(DevassistError::Job(message)) => Err(DevassistError::Job(message.clone())),
            }
        }
    }

    #[test]
    fn view_reports_idle_by_default() {
        let session = CmdKSession::new(Box::new(StubBackend::new(Ok(AiSuggestion {
            command: "x".to_string(),
            explanation: String::new(),
        }))));
        assert_eq!(session.view(), CmdKView::Idle);
    }

    #[test]
    fn view_reports_previewed_command_when_available() {
        let mut session = CmdKSession::new(Box::new(StubBackend::new(Ok(AiSuggestion {
            command: "echo hi".to_string(),
            explanation: String::new(),
        }))));
        let _ = session
            .request_preview(
                "hi",
                AiContext {
                    cwd: "path".to_string(),
                    shell: String::new(),
                    failed_command: None,
                    transcript_tail: String::new(),
                },
            )
            .expect("preview should be produced");
        assert_eq!(
            session.view(),
            CmdKView::Previewed {
                command: "echo hi".to_string()
            }
        );
    }

    #[test]
    fn view_reports_unavailable_reason() {
        let mut session = CmdKSession::new(Box::new(StubBackend::new(Err(
            DevassistError::Unavailable("claude missing".to_string()),
        ))));
        let result = session.request_preview(
            "hi",
            AiContext {
                cwd: "path".to_string(),
                shell: String::new(),
                failed_command: None,
                transcript_tail: String::new(),
            },
        );
        assert!(result.is_err());
        assert_eq!(
            session.view(),
            CmdKView::Unavailable {
                reason: "claude missing".to_string()
            }
        );
    }
}
