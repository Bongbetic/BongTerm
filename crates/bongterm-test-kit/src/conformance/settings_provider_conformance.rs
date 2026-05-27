//! Conformance suite for [`bongterm_settings::SettingsProvider`].

use bongterm_settings::SettingsProvider;

/// Run happy-path conformance checks against any [`SettingsProvider`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(provider: &impl SettingsProvider) {
    let settings = provider.current();
    assert!(
        !settings.keybindings.command_palette.is_empty(),
        "current().keybindings.command_palette must be non-empty"
    );
}
