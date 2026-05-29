//! Replay-with-summarized-context.

use crate::{AgentAdapter, AgentError, AgentExitSummary, ProcessSpec};

/// Re-launch specification derived from prior run summary + original prompt.
#[derive(Debug, Clone)]
pub struct ReplaySpec {
    pub cwd: String,
    pub prefilled_prompt: String,
}

impl ReplaySpec {
    /// Build process spec using same adapter and same cwd.
    pub fn to_process_spec(&self, adapter: &impl AgentAdapter) -> Result<ProcessSpec, AgentError> {
        adapter.build_process_spec(&self.cwd, &self.prefilled_prompt)
    }
}

/// Builds replay specs from original run data.
pub struct ReplayBuilder {
    cwd: String,
    original_prompt: String,
}

impl ReplayBuilder {
    #[must_use]
    pub fn new(cwd: impl Into<String>, original_prompt: impl Into<String>) -> Self {
        Self {
            cwd: cwd.into(),
            original_prompt: original_prompt.into(),
        }
    }

    #[must_use]
    pub fn build(&self, summary: &AgentExitSummary) -> ReplaySpec {
        let prefilled_prompt = match &summary.replay_summary {
            Some(context) => format!(
                "Previous run summary: {context}\n\nOriginal request: {}",
                self.original_prompt
            ),
            None => self.original_prompt.clone(),
        };

        ReplaySpec {
            cwd: self.cwd.clone(),
            prefilled_prompt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentExitSummary, ExitState, MockAgentAdapter};

    fn summary() -> AgentExitSummary {
        AgentExitSummary {
            exit_state: ExitState::Clean { exit_code: 0 },
            tool_calls_made: 4,
            output_bytes: 2048,
            replay_summary: Some("Re-run claude-code (4 tool calls, 2048 bytes)".to_string()),
        }
    }

    #[test]
    fn replay_spec_prefills_prompt_with_summary_context() {
        let spec = ReplayBuilder::new("C:\\repo", "fix the failing test").build(&summary());
        assert_eq!(spec.cwd, "C:\\repo");
        assert!(spec.prefilled_prompt.contains("fix the failing test"));
        assert!(
            spec.prefilled_prompt.contains("Re-run claude-code"),
            "prefilled prompt must carry the exit summary context"
        );
    }

    #[test]
    fn replay_without_summary_still_prefills_original_prompt() {
        let mut s = summary();
        s.replay_summary = None;
        let spec = ReplayBuilder::new("C:\\repo", "do x").build(&s);
        assert!(spec.prefilled_prompt.contains("do x"));
    }

    #[test]
    fn replay_rebuilds_process_spec_with_same_adapter_and_cwd() {
        let adapter = MockAgentAdapter::new("claude-code", Vec::new());
        let spec = ReplayBuilder::new("C:\\repo", "redo").build(&summary());
        let proc = spec.to_process_spec(&adapter).unwrap();
        assert_eq!(proc.launch.cwd.as_deref(), Some("C:\\repo"));
        assert!(proc.launch.argv.iter().any(|a| a.contains("redo")));
    }
}
