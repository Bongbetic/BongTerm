//! Context Optimizer v1 — token-budget tool-schema pruning per agent.
//! Saves tokens, not resident memory.

use std::collections::HashSet;

use crate::{McpServerConfig, McpToolDescriptor};

/// Per-agent allowlist of tool names. Default deny: only listed tools pass.
#[derive(Debug, Clone)]
pub struct ToolAllowlist {
    allowed: HashSet<String>,
}

impl ToolAllowlist {
    #[must_use]
    pub fn new(names: Vec<String>) -> Self {
        Self {
            allowed: names.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.allowed.contains(name)
    }
}

/// Prunes MCP tool schema exposed to agent to token-bounded allowlist.
pub struct ContextOptimizer {
    allowlist: ToolAllowlist,
}

/// Whether an agent adapter supports BongTerm-mediated MCP configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSupport {
    ConfigFile,
    EnvInjection,
    None,
}

/// Token-budget preview of pruned tool schema. Tokens only, never RSS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenBudgetPreview {
    pub exposed_tool_count: usize,
    pub pruned_tool_count: usize,
    pub estimated_tokens: usize,
}

/// Temporary scoped config exposing only allowlisted tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedMcpConfig {
    pub server_name: String,
    pub exposed_tools: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OptimizerError {
    #[error("MCP governance unavailable for this agent adapter")]
    McpGovernanceUnavailable,
}

impl ContextOptimizer {
    #[must_use]
    pub fn new(allowlist: ToolAllowlist) -> Self {
        Self { allowlist }
    }

    #[must_use]
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        self.allowlist.contains(tool_name)
    }

    /// Return only allowlisted tools, preserving input order.
    #[must_use]
    pub fn filter_tools(&self, tools: &[McpToolDescriptor]) -> Vec<McpToolDescriptor> {
        tools
            .iter()
            .filter(|tool| self.allowlist.contains(&tool.name))
            .cloned()
            .collect()
    }

    /// Estimate token budget of pruned schema this agent will see.
    #[must_use]
    pub fn preview(&self, tools: &[McpToolDescriptor]) -> TokenBudgetPreview {
        let exposed = self.filter_tools(tools);
        let chars: usize = exposed
            .iter()
            .map(|tool| tool.name.len() + tool.description.len() + tool.input_schema_json.len())
            .sum();
        TokenBudgetPreview {
            exposed_tool_count: exposed.len(),
            pruned_tool_count: tools.len() - exposed.len(),
            estimated_tokens: chars.div_ceil(4),
        }
    }

    pub fn generate_scoped_config(
        &self,
        support: McpSupport,
        server: &McpServerConfig,
        tools: &[McpToolDescriptor],
    ) -> Result<ScopedMcpConfig, OptimizerError> {
        match support {
            McpSupport::None => Err(OptimizerError::McpGovernanceUnavailable),
            McpSupport::ConfigFile | McpSupport::EnvInjection => Ok(ScopedMcpConfig {
                server_name: server.name.clone(),
                exposed_tools: self
                    .filter_tools(tools)
                    .into_iter()
                    .map(|tool| tool.name)
                    .collect(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{McpServerConfig, McpToolDescriptor};

    fn tool(name: &str) -> McpToolDescriptor {
        McpToolDescriptor {
            name: name.into(),
            description: "d".into(),
            input_schema_json: "{}".into(),
        }
    }

    #[test]
    fn allowlist_filters_tools_default_deny() {
        let all = vec![tool("read_file"), tool("write_file"), tool("delete_all")];
        let allow = ToolAllowlist::new(vec!["read_file".into(), "write_file".into()]);
        let opt = ContextOptimizer::new(allow);
        let exposed = opt.filter_tools(&all);
        let names: Vec<&str> = exposed.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names, vec!["read_file", "write_file"]);
        assert!(!opt.is_allowed("delete_all"));
        assert!(opt.is_allowed("read_file"));
    }

    #[test]
    fn empty_allowlist_blocks_everything() {
        let all = vec![tool("read_file")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec![]));
        assert!(opt.filter_tools(&all).is_empty());
    }

    #[test]
    fn token_budget_preview_counts_only_allowed_tools() {
        let all = vec![tool("read_file"), tool("write_file"), tool("delete_all")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec!["read_file".into()]));
        let preview = opt.preview(&all);
        assert_eq!(preview.exposed_tool_count, 1);
        assert_eq!(preview.pruned_tool_count, 2);
        assert!(preview.estimated_tokens > 0);
        let full = ContextOptimizer::new(ToolAllowlist::new(vec![
            "read_file".into(),
            "write_file".into(),
            "delete_all".into(),
        ]));
        assert!(preview.estimated_tokens < full.preview(&all).estimated_tokens);
    }

    #[test]
    fn generates_scoped_config_for_supporting_agent() {
        let server = McpServerConfig {
            name: "fs".into(),
            argv: vec!["node".into()],
            env: vec![],
        };
        let all = vec![tool("read_file"), tool("write_file"), tool("delete_all")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec!["read_file".into()]));
        let scoped = opt
            .generate_scoped_config(McpSupport::ConfigFile, &server, &all)
            .unwrap();
        assert_eq!(scoped.exposed_tools, vec!["read_file".to_string()]);
        assert_eq!(scoped.server_name, "fs");
    }

    #[test]
    fn non_supporting_agent_is_labeled_unavailable() {
        let server = McpServerConfig {
            name: "fs".into(),
            argv: vec!["node".into()],
            env: vec![],
        };
        let all = vec![tool("read_file")];
        let opt = ContextOptimizer::new(ToolAllowlist::new(vec!["read_file".into()]));
        let err = opt
            .generate_scoped_config(McpSupport::None, &server, &all)
            .unwrap_err();
        assert!(
            matches!(err, OptimizerError::McpGovernanceUnavailable),
            "got {err:?}"
        );
    }
}
