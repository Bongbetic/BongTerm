//! Conformance checks for UIA providers.

use bongterm_ui::accessibility::{AxRole, UiaProvider};

/// # Panics
///
/// Panics when the provider does not expose a named root window with UIA metadata.
pub fn run_uia_provider_conformance(provider: &impl UiaProvider) {
    assert_eq!(
        provider.control_type_of(0),
        Some(AxRole::Window.uia_control_type_id())
    );
    assert!(provider.name_of(0).is_some_and(|name| !name.is_empty()));
    let root = provider.root();
    assert_eq!(root.role, AxRole::Window);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_ui::accessibility::{AxNode, TreeUiaProvider};

    #[test]
    fn model_provider_satisfies_contract() {
        let provider = TreeUiaProvider::new(AxNode::new(AxRole::Window, "BongTerm"));
        run_uia_provider_conformance(&provider);
    }
}
