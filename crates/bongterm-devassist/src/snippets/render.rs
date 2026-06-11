//! Snippet parameter substitution (gate #12).
//!
//! Substitutes every `${param:name}` with the provided value. A missing
//! parameter is an error, never a partial command. Substitution is single-pass
//! so a value that itself contains `${param:...}` is inserted literally and not
//! re-expanded.

use crate::DevassistError;
use crate::snippets::model::Snippet;
use std::collections::HashMap;

/// Render a snippet by substituting all `${param:name}` placeholders.
///
/// Returns [`DevassistError::MissingParam`] naming the first absent parameter.
#[allow(clippy::implicit_hasher)]
pub fn render_snippet(
    snippet: &Snippet,
    params: &HashMap<String, String>,
) -> Result<String, DevassistError> {
    for name in snippet.params() {
        if !params.contains_key(&name) {
            return Err(DevassistError::MissingParam(name));
        }
    }

    let mut out = String::with_capacity(snippet.command.len());
    let cmd = &snippet.command;
    let needle = "${param:";
    let mut rest = cmd.as_str();

    while let Some(pos) = rest.find(needle) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + needle.len()..];
        if let Some(end) = after.find('}') {
            let name = &after[..end];
            out.push_str(params.get(name).map_or("", String::as_str));
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[pos..]);
            rest = "";
            break;
        }
    }

    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snippets::model::{Snippet, SnippetScope};
    use std::collections::HashMap;

    fn snip(cmd: &str) -> Snippet {
        Snippet {
            name: "s".to_string(),
            description: String::new(),
            scope: SnippetScope::Global,
            command: cmd.to_string(),
        }
    }

    #[test]
    fn substitutes_all_params() {
        let s = snip("git checkout ${param:branch}");
        let mut p = HashMap::new();
        p.insert("branch".to_string(), "main".to_string());
        let out = render_snippet(&s, &p).expect("render");
        assert_eq!(out, "git checkout main");
    }

    #[test]
    fn repeated_param_substituted_everywhere() {
        let s = snip("echo ${param:x} ${param:x}");
        let mut p = HashMap::new();
        p.insert("x".to_string(), "hi".to_string());
        assert_eq!(render_snippet(&s, &p).unwrap(), "echo hi hi");
    }

    #[test]
    fn missing_param_is_error_not_partial_run() {
        let s = snip("./deploy.sh ${param:env} ${param:tag}");
        let mut p = HashMap::new();
        p.insert("env".to_string(), "prod".to_string());
        let err = render_snippet(&s, &p).unwrap_err();
        match err {
            crate::DevassistError::MissingParam(name) => assert_eq!(name, "tag"),
            other => panic!("expected MissingParam, got {other:?}"),
        }
    }

    #[test]
    fn value_with_placeholder_syntax_is_not_re_expanded() {
        let s = snip("echo ${param:a}");
        let mut p = HashMap::new();
        p.insert("a".to_string(), "${param:b}".to_string());
        assert_eq!(render_snippet(&s, &p).unwrap(), "echo ${param:b}");
    }
}
