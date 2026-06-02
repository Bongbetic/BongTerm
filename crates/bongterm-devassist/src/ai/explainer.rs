//! Failed-command explainer.
//!
//! Offered only for command blocks with non-zero exit codes. Produces text only;
//! it never executes suggested fixes.

use bongterm_storage_api::CommandBlockRow;

use crate::DevassistError;
use crate::ai::runner::{AiBackend, AiContext, AiIntent, AiRequest, AiSuggestion};

const MAX_TAIL: usize = 4096;

/// Builds and dispatches failed-command explanations.
pub struct Explainer {
    backend: Box<dyn AiBackend>,
}

impl Explainer {
    #[must_use]
    pub fn new(backend: Box<dyn AiBackend>) -> Self {
        Self { backend }
    }

    /// A block is explainable only when it finished with a non-zero exit code.
    #[must_use]
    pub fn is_explainable(block: &CommandBlockRow) -> bool {
        matches!(block.exit_code, Some(code) if code != 0)
    }

    /// Produce an explanation for a failed command block.
    pub fn explain(
        &self,
        block: &CommandBlockRow,
        output_tail: &str,
    ) -> Result<AiSuggestion, DevassistError> {
        if !Self::is_explainable(block) {
            return Err(DevassistError::Parse(
                "block did not fail; nothing to explain".to_string(),
            ));
        }

        let tail = bounded_tail(output_tail);
        let request = AiRequest {
            intent: AiIntent::ExplainFailure,
            user_text: format!(
                "Command `{}` exited with code {}. Explain why and suggest a fix.",
                block.command,
                block.exit_code.unwrap_or_default()
            ),
            context: AiContext {
                cwd: String::new(),
                shell: String::new(),
                failed_command: Some(block.command.clone()),
                transcript_tail: tail.to_string(),
            },
        };

        self.backend.suggest(&request)
    }
}

fn bounded_tail(output_tail: &str) -> &str {
    if output_tail.len() <= MAX_TAIL {
        return output_tail;
    }

    let mut start = output_tail.len() - MAX_TAIL;
    while !output_tail.is_char_boundary(start) {
        start += 1;
    }
    &output_tail[start..]
}
