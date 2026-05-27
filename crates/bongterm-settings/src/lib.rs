//! bongterm-settings
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use arc_swap::ArcSwap;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeybindingSettings {
    pub command_palette: String,
    pub new_tab: String,
    pub close_pane: String,
}

impl Default for KeybindingSettings {
    fn default() -> Self {
        Self {
            command_palette: "Ctrl+Shift+P".to_string(),
            new_tab: "Ctrl+Shift+T".to_string(),
            close_pane: "Ctrl+Shift+W".to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThemeSettings {
    pub name: String,
    pub font_family: String,
    pub font_size: f32,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            name: "dark".to_string(),
            font_family: "Cascadia Code".to_string(),
            font_size: 13.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Settings {
    pub keybindings: KeybindingSettings,
    pub theme: ThemeSettings,
}

/// Port interface for reading application settings.
/// The snapshot returned by `current()` is immutable — settings changes produce a new snapshot.
pub trait SettingsProvider: Send + Sync {
    /// Returns the current settings snapshot.
    fn current(&self) -> Arc<Settings>;

    /// Subscribe to settings changes. Returns a receiver that fires when settings reload.
    fn subscribe(&self) -> tokio::sync::watch::Receiver<Arc<Settings>>;
}

pub struct MockSettingsProvider {
    settings: ArcSwap<Settings>,
    tx: tokio::sync::watch::Sender<Arc<Settings>>,
    rx: tokio::sync::watch::Receiver<Arc<Settings>>,
}

impl MockSettingsProvider {
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        let arc = Arc::new(settings);
        let (tx, rx) = tokio::sync::watch::channel(arc.clone());
        Self {
            settings: ArcSwap::new(arc),
            tx,
            rx,
        }
    }

    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(Settings::default())
    }

    /// Push a new settings snapshot (simulates a settings reload).
    pub fn reload(&self, settings: Settings) {
        let arc = Arc::new(settings);
        self.settings.store(arc.clone());
        let _ = self.tx.send(arc);
    }
}

impl SettingsProvider for MockSettingsProvider {
    fn current(&self) -> Arc<Settings> {
        self.settings.load_full()
    }

    fn subscribe(&self) -> tokio::sync::watch::Receiver<Arc<Settings>> {
        self.rx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_command_palette_keybinding() {
        let provider = MockSettingsProvider::with_defaults();
        let snap = provider.current();
        assert_eq!(snap.keybindings.command_palette, "Ctrl+Shift+P");
    }

    #[test]
    fn reload_updates_snapshot() {
        let provider = MockSettingsProvider::with_defaults();
        let new_settings = Settings {
            keybindings: KeybindingSettings {
                command_palette: "Ctrl+P".to_string(),
                ..KeybindingSettings::default()
            },
            ..Settings::default()
        };
        provider.reload(new_settings);
        assert_eq!(provider.current().keybindings.command_palette, "Ctrl+P");
    }

    #[test]
    fn settings_serde_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(
            s.keybindings.command_palette,
            s2.keybindings.command_palette
        );
    }
}
