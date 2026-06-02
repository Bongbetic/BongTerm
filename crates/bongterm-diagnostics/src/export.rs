//! Diagnostic export bundle with mandatory redaction preview.

use bongterm_security::redactor::{RedactionPreview, Redactor};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticItem {
    pub name: String,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticBundle {
    pub items: Vec<DiagnosticItem>,
    pub preview: RedactionPreview,
}

impl DiagnosticBundle {
    #[must_use]
    pub fn build(items: Vec<DiagnosticItem>, redactor: &Redactor) -> Self {
        let joined = items
            .iter()
            .map(|item| format!("== {} ==\n{}", item.name, item.contents))
            .collect::<Vec<_>>()
            .join("\n");
        let preview = redactor.preview(&joined);
        let items = items
            .into_iter()
            .map(|item| DiagnosticItem {
                name: item.name,
                contents: redactor.redact(&item.contents),
            })
            .collect();
        Self { items, preview }
    }

    #[must_use]
    pub fn contains_plaintext(&self, needle: &str) -> bool {
        self.items.iter().any(|item| item.contents.contains(needle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_redacts_secret_before_export() {
        let redactor = Redactor;
        let bundle = DiagnosticBundle::build(
            vec![DiagnosticItem {
                name: "env".to_string(),
                contents: "OPENAI_API_KEY=sk-test1234567890abcdef".to_string(),
            }],
            &redactor,
        );
        assert!(bundle.preview.match_count > 0);
        assert!(!bundle.contains_plaintext("sk-test1234567890abcdef"));
    }
}
