//! Cmd-K session: natural-language command assist, preview-only.
//!
//! The session never spawns a process. `confirm_run` only releases the previewed
//! command string after explicit user confirmation.

use crate::DevassistError;
use crate::ai::runner::{AiBackend, AiContext, AiIntent, AiRequest, AiSuggestion};

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
}

impl CmdKSession {
    #[must_use]
    pub fn new(backend: Box<dyn AiBackend>) -> Self {
        Self {
            backend,
            state: CmdKState::Idle,
            last_suggestion: None,
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
                self.state = CmdKState::Previewed;
                Ok(suggestion)
            }
            Err(DevassistError::Unavailable(reason)) => {
                self.state = CmdKState::Unavailable;
                self.last_suggestion = None;
                Err(CmdKError::Unavailable(reason))
            }
            Err(other) => {
                self.state = CmdKState::Idle;
                self.last_suggestion = None;
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
}
