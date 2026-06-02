//! Snippet model: JSON5 library load + `${param:name}` placeholder parsing.
//!
//! Gate #12. Scope is workspace + global. Placeholder parsing is robust:
//! malformed `${param:...}` yields no parameter rather than panicking.

use crate::DevassistError;

/// Where a snippet is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetScope {
    /// Available in every workspace.
    Global,
    /// Available only in the current workspace.
    Workspace,
}

/// A single snippet definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snippet {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub scope: SnippetScope,
    pub command: String,
}

impl Snippet {
    /// Distinct parameter names in first-appearance order, parsed from
    /// `${param:name}` placeholders. Malformed placeholders are ignored.
    #[must_use]
    pub fn params(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let bytes = self.command.as_bytes();
        let needle = b"${param:";
        let mut i = 0;

        while i + needle.len() <= bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                let start = i + needle.len();
                if let Some(rel_end) = self.command[start..].find('}') {
                    let name = &self.command[start..start + rel_end];
                    if !name.is_empty() && !out.iter().any(|existing| existing == name) {
                        out.push(name.to_string());
                    }
                    i = start + rel_end + 1;
                    continue;
                }
                break;
            }
            i += 1;
        }

        out
    }
}

/// A loaded library of snippets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnippetLibrary {
    #[serde(default)]
    pub snippets: Vec<Snippet>,
}

impl SnippetLibrary {
    /// Parse a JSON5 library document.
    pub fn from_json5(text: &str) -> Result<Self, DevassistError> {
        json5::from_str(text).map_err(|e| DevassistError::Parse(format!("snippet json5: {e}")))
    }

    /// Snippets visible in the given scope.
    #[must_use]
    pub fn visible_in(&self, scope: SnippetScope) -> Vec<&Snippet> {
        self.snippets
            .iter()
            .filter(|snippet| snippet.scope == SnippetScope::Global || snippet.scope == scope)
            .collect()
    }
}

/// The parameter-prompt model the UI renders before running a snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamPrompt {
    pub snippet_name: String,
    /// Parameter names to collect, in order.
    pub params: Vec<String>,
}

/// Merged snippet store: workspace snippets override global by name.
#[derive(Debug, Clone)]
pub struct SnippetStore {
    global: SnippetLibrary,
    workspace: SnippetLibrary,
}

impl SnippetStore {
    #[must_use]
    pub fn new(global: SnippetLibrary, workspace: SnippetLibrary) -> Self {
        Self { global, workspace }
    }

    /// Resolve a snippet by name. Workspace scope wins over global.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<&Snippet> {
        self.workspace
            .snippets
            .iter()
            .find(|s| s.name == name)
            .or_else(|| self.global.snippets.iter().find(|s| s.name == name))
    }

    /// All distinct snippet names (workspace + global).
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for s in self
            .workspace
            .snippets
            .iter()
            .chain(self.global.snippets.iter())
        {
            if !out.iter().any(|n| n == &s.name) {
                out.push(s.name.clone());
            }
        }
        out
    }

    /// Build the parameter prompt for a snippet, or `None` if it does not exist.
    #[must_use]
    pub fn prompt_for(&self, name: &str) -> Option<ParamPrompt> {
        self.resolve(name).map(|s| ParamPrompt {
            snippet_name: s.name.clone(),
            params: s.params(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIB_JSON5: &str = r#"
    {
        // global snippets
        snippets: [
            {
                name: "gco",
                description: "git checkout a branch",
                scope: "global",
                command: "git checkout ${param:branch}",
            },
            {
                name: "deploy",
                description: "deploy to an env",
                scope: "workspace",
                command: "./deploy.sh ${param:env} ${param:tag}",
            },
        ],
    }
    "#;

    #[test]
    fn loads_json5_library_with_comments() {
        let lib = SnippetLibrary::from_json5(LIB_JSON5).expect("parse json5");
        assert_eq!(lib.snippets.len(), 2);
        assert_eq!(lib.snippets[0].name, "gco");
        assert_eq!(lib.snippets[0].scope, SnippetScope::Global);
        assert_eq!(lib.snippets[1].scope, SnippetScope::Workspace);
    }

    #[test]
    fn parses_params_in_order_without_duplicates() {
        let snip = Snippet {
            name: "deploy".to_string(),
            description: String::new(),
            scope: SnippetScope::Workspace,
            command: "./deploy.sh ${param:env} ${param:tag} ${param:env}".to_string(),
        };
        let params = snip.params();
        assert_eq!(params, vec!["env".to_string(), "tag".to_string()]);
    }

    #[test]
    fn malformed_json5_is_a_parse_error() {
        let err = SnippetLibrary::from_json5("{ snippets: [ { name: ").unwrap_err();
        assert!(matches!(err, crate::DevassistError::Parse(_)));
    }

    #[test]
    fn malformed_placeholder_is_rejected() {
        let snip = Snippet {
            name: "x".to_string(),
            description: String::new(),
            scope: SnippetScope::Global,
            command: "echo ${param:unterminated".to_string(),
        };
        assert!(snip.params().is_empty());
    }

    #[test]
    fn store_merges_global_and_workspace_with_workspace_priority() {
        let global = SnippetLibrary::from_json5(
            r#"{ snippets: [ { name: "ls", scope: "global", command: "ls -la" } ] }"#,
        )
        .unwrap();
        let workspace = SnippetLibrary::from_json5(
            r#"{ snippets: [ { name: "ls", scope: "workspace", command: "exa -la" },
                            { name: "t", scope: "workspace", command: "cargo test" } ] }"#,
        )
        .unwrap();
        let store = SnippetStore::new(global, workspace);
        assert_eq!(store.resolve("ls").unwrap().command, "exa -la");
        assert_eq!(store.resolve("t").unwrap().command, "cargo test");
        assert!(store.resolve("nope").is_none());
        let mut names = store.names();
        names.sort();
        assert_eq!(names, vec!["ls".to_string(), "t".to_string()]);
    }

    #[test]
    fn prompt_lists_params_needing_input() {
        let store = SnippetStore::new(
            SnippetLibrary { snippets: vec![] },
            SnippetLibrary::from_json5(
                r#"{ snippets: [ { name: "d", scope: "workspace", command: "./d ${param:env} ${param:tag}" } ] }"#,
            )
            .unwrap(),
        );
        let prompt = store.prompt_for("d").expect("snippet exists");
        assert_eq!(prompt.snippet_name, "d");
        assert_eq!(prompt.params, vec!["env".to_string(), "tag".to_string()]);
    }
}
