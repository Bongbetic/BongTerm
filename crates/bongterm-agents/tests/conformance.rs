//! Conformance: both production adapters satisfy the AgentAdapter contract.
use bongterm_agents::claude_code::ClaudeCodeAdapter;
use bongterm_agents::codex_cli::CodexCliAdapter;
use bongterm_test_kit::conformance::agent_adapter_conformance;

#[test]
fn claude_code_adapter_conforms() {
    agent_adapter_conformance::run_offline(&ClaudeCodeAdapter::new());
}

#[test]
fn codex_cli_adapter_conforms() {
    agent_adapter_conformance::run_offline(&CodexCliAdapter::new());
}
