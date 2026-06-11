//! bongterm-settings
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use arc_swap::ArcSwap;
use schemars::JsonSchema;
use std::path::PathBuf;
use std::sync::Arc;

/// Current on-disk settings schema version.
///
/// Bump this whenever the [`Settings`] shape changes in a way that needs a
/// migration step, and add the corresponding arm to [`migrate`].
///
/// History:
/// - implicit `1` — original shape (no `schema_version` key on disk).
/// - `2` — added the explicit `schema_version` field.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// The version assumed for an on-disk document that predates the
/// `schema_version` key entirely (i.e. the key is absent).
const LEGACY_SCHEMA_VERSION: u32 = 1;

/// serde `default` for [`Settings::schema_version`]. A document deserialized
/// directly (bypassing migration) is assumed to already be current.
fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(default)]
pub struct KeybindingSettings {
    // Phase 1
    pub command_palette: String,
    pub new_tab: String,
    pub close_pane: String,
    pub split_pane: String,
    pub find_in_pane: String,
    pub open_resource_dashboard: String,
    // Phase 3 stubs — stored in settings but UI renders them disabled until Phase 3
    pub cmd_k: String,
    pub smart_history: String,
    pub explain_last_failed: String,
    pub attach_context: String,
    pub toggle_background_jobs: String,
}

impl Default for KeybindingSettings {
    fn default() -> Self {
        Self {
            command_palette: "Ctrl+Shift+P".to_string(),
            new_tab: "Ctrl+Shift+T".to_string(),
            close_pane: "Ctrl+Shift+W".to_string(),
            split_pane: "Alt+Shift+D".to_string(),
            find_in_pane: "Ctrl+F".to_string(),
            open_resource_dashboard: "Ctrl+Shift+R".to_string(),
            cmd_k: "Ctrl+K".to_string(),
            smart_history: "Ctrl+R".to_string(),
            explain_last_failed: "Ctrl+Shift+E".to_string(),
            attach_context: "Ctrl+Shift+A".to_string(),
            toggle_background_jobs: "Ctrl+Shift+J".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(default)]
pub struct ThemeSettings {
    pub name: String,
    pub font_family: String,
    pub font_size: f32,
    pub contrast: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            name: "dark".to_string(),
            font_family: "Cascadia Code".to_string(),
            font_size: 13.0,
            contrast: "normal".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(default)]
pub struct OnboardingSettings {
    pub completed: bool,
    pub default_shell: String,
    pub shell_integration_enabled: bool,
    pub telemetry_enabled: bool,
}

impl Default for OnboardingSettings {
    fn default() -> Self {
        Self {
            completed: false,
            default_shell: "powershell".to_string(),
            shell_integration_enabled: true,
            telemetry_enabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(default)]
pub struct Settings {
    /// On-disk schema version. Drives [`migrate`]. Absent on legacy documents
    /// (treated as [`LEGACY_SCHEMA_VERSION`]); a document deserialized directly
    /// is assumed current via [`default_schema_version`].
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub keybindings: KeybindingSettings,
    pub theme: ThemeSettings,
    pub onboarding: OnboardingSettings,
}

impl Default for Settings {
    fn default() -> Self {
        // Hand-written (not derived) so a fresh `Settings` is stamped at the
        // current schema version. A derived `Default` would yield
        // `schema_version == 0`, which is not a real version and would make the
        // migration/recovery logic ambiguous.
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            keybindings: KeybindingSettings::default(),
            theme: ThemeSettings::default(),
            onboarding: OnboardingSettings::default(),
        }
    }
}

impl Settings {
    pub fn from_json5_str(raw: &str) -> Result<Self, SettingsError> {
        json5::from_str(raw).map_err(|source| SettingsError::Parse {
            message: source.to_string(),
        })
    }

    /// Parse a JSON5 document, run schema migration, and deserialize into
    /// [`Settings`].
    ///
    /// This is the version-aware counterpart to [`from_json5_str`]. It first
    /// reads the document as a free-form [`serde_json::Value`] so the on-disk
    /// `schema_version` can be inspected *before* it is masked by serde
    /// defaults, runs [`migrate`] to bring it forward to
    /// [`CURRENT_SCHEMA_VERSION`], then deserializes the migrated value. Any
    /// fields the user set that are still part of the current shape are
    /// preserved across migration.
    ///
    /// [`from_json5_str`]: Settings::from_json5_str
    pub fn from_json5_str_migrating(raw: &str) -> Result<Self, SettingsError> {
        let value: serde_json::Value =
            json5::from_str(raw).map_err(|source| SettingsError::Parse {
                message: source.to_string(),
            })?;
        let from_version = document_schema_version(&value);
        migrate(value, from_version)
    }
}

/// Read the `schema_version` from a raw settings document.
///
/// A document that predates the field entirely (key absent or not an integer)
/// is treated as [`LEGACY_SCHEMA_VERSION`]. This must read the raw value rather
/// than a deserialized [`Settings`], because the `#[serde(default)]` container
/// attribute would otherwise fill an absent version with the *current* version
/// and erase the legacy signal.
#[must_use]
fn document_schema_version(value: &serde_json::Value) -> u32 {
    value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(LEGACY_SCHEMA_VERSION)
}

/// Migrate a raw settings document from `from_version` up to
/// [`CURRENT_SCHEMA_VERSION`], then deserialize it into [`Settings`].
///
/// Migration is performed on the untyped [`serde_json::Value`] so that:
/// 1. user-set fields are preserved (we never throw the document away), and
/// 2. new fields introduced by a later version are filled with their defaults
///    by serde when the (now-current) value is finally deserialized.
///
/// The function walks one version at a time via a `match` on the running
/// version, so future version steps slot in as additional arms without
/// disturbing earlier ones. A document already at (or somehow ahead of) the
/// current version is deserialized unchanged.
///
/// # Errors
///
/// Returns [`SettingsError::Parse`] if the migrated value does not deserialize
/// into a valid [`Settings`].
pub fn migrate(mut value: serde_json::Value, from_version: u32) -> Result<Settings, SettingsError> {
    let mut version = from_version;
    while version < CURRENT_SCHEMA_VERSION {
        match version {
            // v1 -> v2: the `schema_version` field was introduced. Older
            // documents simply lack it; every other field carries forward
            // untouched and any genuinely new field is materialized from its
            // serde default at the final deserialize step below.
            1 => {
                version = 2;
            }
            // Defensive: an unknown intermediate version cannot be stepped.
            // Stamp it forward so the loop terminates; the final deserialize
            // still validates the resulting document.
            _ => {
                version = CURRENT_SCHEMA_VERSION;
            }
        }
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "schema_version".to_string(),
                serde_json::Value::from(version),
            );
        }
    }

    // Ensure the version stamp is present even when no step ran (e.g. the
    // document was already current but omitted the key).
    if let Some(obj) = value.as_object_mut() {
        obj.entry("schema_version")
            .or_insert_with(|| serde_json::Value::from(CURRENT_SCHEMA_VERSION));
    }

    serde_json::from_value(value).map_err(|source| SettingsError::Parse {
        message: source.to_string(),
    })
}

/// Result of a recovery-aware settings load (see
/// [`FileSettingsProvider::load_with_recovery`]).
///
/// When the on-disk file is missing, this reports normal defaults
/// (`safe_mode == false`, `backup_path == None`) — a missing file is not an
/// error. When the file is present but fails to parse or validate, the original
/// bytes are backed up, [`Settings::default`] is loaded, and `safe_mode` is set
/// so the caller can surface "running on defaults because your config was bad".
#[derive(Debug, Clone)]
pub struct LoadOutcome {
    /// The settings to run with. Either the loaded/migrated user settings, or
    /// [`Settings::default`] when Safe Mode was triggered.
    pub settings: Settings,
    /// `true` when the on-disk config was present but unusable and defaults
    /// were substituted. `false` for a clean load, a successful migration, or a
    /// missing file.
    pub safe_mode: bool,
    /// Path the original (bad) config was copied to before Safe Mode kicked in,
    /// or `None` when no backup was made.
    pub backup_path: Option<PathBuf>,
}

/// Returns the JSON Schema for [`Settings`] as a [`serde_json::Value`].
///
/// # Panics
///
/// Panics if the generated schema cannot be converted to a
/// [`serde_json::Value`], which should never happen for the static schema.
#[must_use]
pub fn settings_schema_json() -> serde_json::Value {
    let schema = schemars::schema_for!(Settings);
    serde_json::to_value(schema).expect("settings schema must serialize")
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings reload failed: {message}")]
    Parse { message: String },

    #[error("settings I/O failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Port interface for reading application settings.
/// The snapshot returned by `current()` is immutable — settings changes produce a new snapshot.
pub trait SettingsProvider: Send + Sync {
    /// Returns the current settings snapshot.
    fn current(&self) -> Arc<Settings>;

    /// Subscribe to settings changes. Returns a receiver that fires when settings reload.
    fn subscribe(&self) -> tokio::sync::watch::Receiver<Arc<Settings>>;
}

pub struct FileSettingsProvider {
    path: PathBuf,
    settings: ArcSwap<Settings>,
    tx: tokio::sync::watch::Sender<Arc<Settings>>,
    rx: tokio::sync::watch::Receiver<Arc<Settings>>,
}

impl FileSettingsProvider {
    pub fn load_or_default(path: impl Into<PathBuf>) -> Result<Self, SettingsError> {
        let path = path.into();
        let settings = if path.exists() {
            Self::read_settings(&path)?
        } else {
            Settings::default()
        };
        let arc = Arc::new(settings);
        let (tx, rx) = tokio::sync::watch::channel(arc.clone());

        Ok(Self {
            path,
            settings: ArcSwap::new(arc),
            tx,
            rx,
        })
    }

    pub fn reload_from_disk(&self) -> Result<(), SettingsError> {
        let settings = Self::read_settings(&self.path)?;
        let arc = Arc::new(settings);
        self.settings.store(arc.clone());
        let _ = self.tx.send(arc);
        Ok(())
    }

    fn read_settings(path: &std::path::Path) -> Result<Settings, SettingsError> {
        let raw = std::fs::read_to_string(path).map_err(|source| SettingsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Settings::from_json5_str_migrating(&raw)
    }

    /// Recovery-aware load. Use this at application startup instead of
    /// [`load_or_default`] when you want backup + Safe Mode fallback semantics.
    ///
    /// Behavior:
    /// - **Missing file** → normal [`Settings::default`], `safe_mode == false`,
    ///   `backup_path == None` (a missing config is not an error).
    /// - **Present and valid** (after migration) → the loaded settings,
    ///   `safe_mode == false`, `backup_path == None`.
    /// - **Present but unreadable / unparseable / invalid** → the original bytes
    ///   are copied to a sibling `*.corrupt.bak` file (never clobbering an
    ///   existing backup), [`Settings::default`] is returned with
    ///   `safe_mode == true`, and `backup_path` points at the backup. The
    ///   original file is left byte-for-byte intact.
    ///
    /// This never fails, never panics, and never overwrites the user's bad file
    /// in place — a bad config degrades to Safe Mode rather than erroring, which
    /// is why (unlike [`load_or_default`]) it is infallible.
    ///
    /// [`load_or_default`]: FileSettingsProvider::load_or_default
    #[must_use]
    pub fn load_with_recovery(path: impl Into<PathBuf>) -> Self {
        let (provider, _outcome) = Self::load_with_recovery_reporting(path);
        provider
    }

    /// Like [`load_with_recovery`], but also returns the [`LoadOutcome`] so the
    /// caller can inspect Safe Mode / backup status alongside the provider.
    ///
    /// [`load_with_recovery`]: FileSettingsProvider::load_with_recovery
    #[must_use]
    pub fn load_with_recovery_reporting(path: impl Into<PathBuf>) -> (Self, LoadOutcome) {
        let path = path.into();
        let outcome = compute_load_outcome(&path);

        let arc = Arc::new(outcome.settings.clone());
        let (tx, rx) = tokio::sync::watch::channel(arc.clone());
        let provider = Self {
            path,
            settings: ArcSwap::new(arc),
            tx,
            rx,
        };
        (provider, outcome)
    }
}

/// Compute the recovery [`LoadOutcome`] for `path` without building a provider.
///
/// - Missing file → defaults, not Safe Mode (a missing config is not an error).
/// - Present and parseable (after migration) → the loaded settings.
/// - Present but unreadable/unparseable/invalid → back up the original bytes and
///   fall back to Safe Mode defaults. A backup failure degrades to
///   "no backup recorded" rather than taking down startup.
fn compute_load_outcome(path: &std::path::Path) -> LoadOutcome {
    if !path.exists() {
        return LoadOutcome {
            settings: Settings::default(),
            safe_mode: false,
            backup_path: None,
        };
    }

    if let Ok(settings) = FileSettingsProvider::read_settings(path) {
        LoadOutcome {
            settings,
            safe_mode: false,
            backup_path: None,
        }
    } else {
        let backup_path = backup_corrupt_file(path).ok();
        LoadOutcome {
            settings: Settings::default(),
            safe_mode: true,
            backup_path,
        }
    }
}

/// Copy a corrupt settings file to a sibling backup, returning the backup path.
///
/// The backup is byte-exact (so it preserves non-UTF-8 garbage) and never
/// clobbers an existing backup: it appends `.corrupt.bak`, then
/// `.corrupt.1.bak`, `.corrupt.2.bak`, … until it finds a free name. The
/// search is deterministic (lowest free index wins), which keeps it testable.
///
/// # Errors
///
/// Returns [`SettingsError::Io`] if the source cannot be read or the backup
/// cannot be written.
fn backup_corrupt_file(path: &std::path::Path) -> Result<PathBuf, SettingsError> {
    let backup = next_backup_path(path);
    // `copy` preserves the exact bytes, including invalid UTF-8 that a
    // read-to-string would reject.
    std::fs::copy(path, &backup).map_err(|source| SettingsError::Io {
        path: backup.clone(),
        source,
    })?;
    Ok(backup)
}

/// Compute the first free backup path for `path` (see [`backup_corrupt_file`]).
///
/// `settings.json5` → `settings.json5.corrupt.bak`, then
/// `settings.json5.corrupt.1.bak`, `settings.json5.corrupt.2.bak`, …
#[must_use]
fn next_backup_path(path: &std::path::Path) -> PathBuf {
    let base = path.as_os_str().to_os_string();

    let first = {
        let mut name = base.clone();
        name.push(".corrupt.bak");
        PathBuf::from(name)
    };
    if !first.exists() {
        return first;
    }

    // First backup is taken; find the lowest free numbered suffix.
    let mut index: u32 = 1;
    loop {
        let mut name = base.clone();
        name.push(format!(".corrupt.{index}.bak"));
        let candidate = PathBuf::from(name);
        if !candidate.exists() {
            return candidate;
        }
        index = index.saturating_add(1);
    }
}

impl SettingsProvider for FileSettingsProvider {
    fn current(&self) -> Arc<Settings> {
        self.settings.load_full()
    }

    fn subscribe(&self) -> tokio::sync::watch::Receiver<Arc<Settings>> {
        self.rx.clone()
    }
}

// ─── SettingsWriter port ─────────────────────────────────────────────────────

/// Port interface for persisting application settings to a backing store.
///
/// Real implementation: [`FileSettingsProvider`].
/// Test double: [`MockSettingsWriter`].
pub trait SettingsWriter: Send + Sync {
    /// Serialize `settings` and write atomically to the backing store.
    ///
    /// On success the on-disk representation reflects the new value.
    ///
    /// # Errors
    ///
    /// Returns [`SettingsError::Io`] if the write or rename fails.
    fn write(&self, settings: &Settings) -> Result<(), SettingsError>;
}

impl SettingsWriter for FileSettingsProvider {
    fn write(&self, settings: &Settings) -> Result<(), SettingsError> {
        let json = serde_json::to_string_pretty(settings).expect("Settings must serialize to JSON");
        // Atomic: write to tmp then rename so readers never see a partial file.
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json.as_bytes()).map_err(|source| SettingsError::Io {
            path: tmp.clone(),
            source,
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|source| SettingsError::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(())
    }
}

/// Test double for [`SettingsWriter`]. Records every call for assertion in tests.
pub struct MockSettingsWriter {
    write_calls: Arc<std::sync::Mutex<Vec<Settings>>>,
    fail: Arc<std::sync::Mutex<bool>>,
}

impl MockSettingsWriter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            write_calls: Arc::new(std::sync::Mutex::new(Vec::new())),
            fail: Arc::new(std::sync::Mutex::new(false)),
        }
    }

    /// Configure the mock to return an error on every subsequent `write` call.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn set_fail(&self, should_fail: bool) {
        *self.fail.lock().expect("lock not poisoned") = should_fail;
    }

    /// All [`Settings`] values passed to [`SettingsWriter::write`] in call order.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn write_calls(&self) -> Vec<Settings> {
        self.write_calls.lock().expect("lock not poisoned").clone()
    }
}

impl Default for MockSettingsWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsWriter for MockSettingsWriter {
    fn write(&self, settings: &Settings) -> Result<(), SettingsError> {
        if *self.fail.lock().expect("lock not poisoned") {
            return Err(SettingsError::Io {
                path: PathBuf::from("<mock>"),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "mock write failure",
                ),
            });
        }
        self.write_calls
            .lock()
            .expect("lock not poisoned")
            .push(settings.clone());
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────

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

    // Exact-value comparison is intended: the parsed font_size must equal the
    // literal 15 written in the JSON5 source.
    #[allow(clippy::float_cmp)]
    #[test]
    fn parses_json5_settings() {
        let raw = r#"
        {
          // JSON5 comments are allowed.
          keybindings: {
            command_palette: "Ctrl+P",
            new_tab: "Ctrl+Shift+T",
            close_pane: "Ctrl+Shift+W",
          },
          theme: {
            name: "high-contrast",
            font_family: "Cascadia Mono",
            font_size: 15,
          },
        }
        "#;

        let settings = Settings::from_json5_str(raw).unwrap();

        assert_eq!(settings.keybindings.command_palette, "Ctrl+P");
        assert_eq!(settings.theme.name, "high-contrast");
        assert_eq!(settings.theme.font_size, 15.0);
    }

    #[test]
    fn file_provider_keeps_last_valid_snapshot_after_reload_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json5");
        std::fs::write(
            &path,
            r#"{ keybindings: { command_palette: "Ctrl+Alt+P", new_tab: "Ctrl+Shift+T", close_pane: "Ctrl+Shift+W" }, theme: { name: "dark", font_family: "Cascadia Mono", font_size: 13 } }"#,
        )
        .unwrap();
        let provider = FileSettingsProvider::load_or_default(path.clone()).unwrap();
        assert_eq!(provider.current().keybindings.command_palette, "Ctrl+Alt+P");

        std::fs::write(&path, "{ keybindings: ").unwrap();
        let err = provider.reload_from_disk().unwrap_err();

        assert!(err.to_string().contains("settings reload failed"));
        assert_eq!(provider.current().keybindings.command_palette, "Ctrl+Alt+P");
    }

    #[test]
    fn generated_schema_contains_settings_properties() {
        let schema = settings_schema_json();
        let text = serde_json::to_string(&schema).unwrap();

        assert!(text.contains("keybindings"));
        assert!(text.contains("theme"));
        assert!(text.contains("command_palette"));
    }

    #[test]
    fn all_keybinding_defaults_match_ux_contract() {
        let k = KeybindingSettings::default();
        // Phase 1
        assert_eq!(k.command_palette, "Ctrl+Shift+P");
        assert_eq!(k.new_tab, "Ctrl+Shift+T");
        assert_eq!(k.close_pane, "Ctrl+Shift+W");
        assert_eq!(k.split_pane, "Alt+Shift+D");
        assert_eq!(k.find_in_pane, "Ctrl+F");
        assert_eq!(k.open_resource_dashboard, "Ctrl+Shift+R");
        // Phase 3 stubs stored in settings but disabled in UI
        assert_eq!(k.cmd_k, "Ctrl+K");
        assert_eq!(k.smart_history, "Ctrl+R");
        assert_eq!(k.explain_last_failed, "Ctrl+Shift+E");
        assert_eq!(k.attach_context, "Ctrl+Shift+A");
        assert_eq!(k.toggle_background_jobs, "Ctrl+Shift+J");
    }

    #[test]
    fn new_keybinding_fields_survive_serde_roundtrip() {
        let original = KeybindingSettings::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: KeybindingSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(original.split_pane, restored.split_pane);
        assert_eq!(original.find_in_pane, restored.find_in_pane);
        assert_eq!(
            original.open_resource_dashboard,
            restored.open_resource_dashboard
        );
        assert_eq!(original.cmd_k, restored.cmd_k);
        assert_eq!(original.smart_history, restored.smart_history);
        assert_eq!(original.explain_last_failed, restored.explain_last_failed);
        assert_eq!(original.attach_context, restored.attach_context);
        assert_eq!(
            original.toggle_background_jobs,
            restored.toggle_background_jobs
        );
    }

    #[test]
    fn generated_schema_contains_all_keybinding_fields() {
        let schema = settings_schema_json();
        let text = serde_json::to_string(&schema).unwrap();
        assert!(text.contains("split_pane"), "schema missing split_pane");
        assert!(text.contains("find_in_pane"), "schema missing find_in_pane");
        assert!(
            text.contains("open_resource_dashboard"),
            "schema missing open_resource_dashboard"
        );
        assert!(text.contains("cmd_k"), "schema missing cmd_k");
        assert!(
            text.contains("smart_history"),
            "schema missing smart_history"
        );
        assert!(
            text.contains("explain_last_failed"),
            "schema missing explain_last_failed"
        );
        assert!(
            text.contains("attach_context"),
            "schema missing attach_context"
        );
        assert!(
            text.contains("toggle_background_jobs"),
            "schema missing toggle_background_jobs"
        );
    }

    #[test]
    fn onboarding_settings_not_completed_by_default() {
        let s = Settings::default();
        assert!(!s.onboarding.completed);
    }

    #[test]
    fn onboarding_settings_telemetry_off_by_default() {
        let s = Settings::default();
        assert!(!s.onboarding.telemetry_enabled);
    }

    #[test]
    fn onboarding_settings_default_shell_is_powershell() {
        let s = Settings::default();
        assert_eq!(s.onboarding.default_shell, "powershell");
    }

    #[test]
    fn onboarding_settings_shell_integration_on_by_default() {
        let s = Settings::default();
        assert!(s.onboarding.shell_integration_enabled);
    }

    // --- SettingsWriter ---

    #[test]
    fn file_provider_write_persists_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json5");
        let provider = FileSettingsProvider::load_or_default(path.clone()).unwrap();

        let mut new_settings = Settings::default();
        new_settings.keybindings.command_palette = "Ctrl+Alt+P".to_string();

        provider.write(&new_settings).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            raw.contains("Ctrl+Alt+P"),
            "written file must contain new keybinding"
        );
    }

    #[test]
    fn file_provider_write_then_reload_roundtrips_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json5");
        let provider = FileSettingsProvider::load_or_default(path.clone()).unwrap();

        let mut new_settings = Settings::default();
        new_settings.theme.name = "high-contrast".to_string();

        provider.write(&new_settings).unwrap();
        provider.reload_from_disk().unwrap();

        assert_eq!(provider.current().theme.name, "high-contrast");
    }

    #[test]
    fn mock_writer_records_write_calls() {
        let mock = MockSettingsWriter::new();
        let s1 = Settings::default();
        let mut s2 = Settings::default();
        s2.theme.name = "light".to_string();

        mock.write(&s1).unwrap();
        mock.write(&s2).unwrap();

        let calls = mock.write_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].theme.name, "light");
    }

    #[test]
    fn mock_writer_returns_error_when_set_to_fail() {
        let mock = MockSettingsWriter::new();
        mock.set_fail(true);

        let err = mock.write(&Settings::default()).unwrap_err();
        assert!(
            err.to_string().contains("I/O"),
            "error message must describe I/O failure, got: {err}"
        );
        // no calls recorded on failure
        assert!(mock.write_calls().is_empty());
    }

    #[test]
    fn mock_writer_default_succeeds() {
        let mock = MockSettingsWriter::default();
        mock.write(&Settings::default()).unwrap();
        assert_eq!(mock.write_calls().len(), 1);
    }

    #[test]
    fn onboarding_settings_survive_serde_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s.onboarding.completed, s2.onboarding.completed);
        assert_eq!(
            s.onboarding.telemetry_enabled,
            s2.onboarding.telemetry_enabled
        );
        assert_eq!(s.onboarding.default_shell, s2.onboarding.default_shell);
        assert_eq!(
            s.onboarding.shell_integration_enabled,
            s2.onboarding.shell_integration_enabled
        );
    }

    #[test]
    fn generated_schema_contains_onboarding_section() {
        let schema = settings_schema_json();
        let text = serde_json::to_string(&schema).unwrap();
        assert!(text.contains("onboarding"));
        assert!(text.contains("telemetry_enabled"));
        assert!(text.contains("default_shell"));
        assert!(text.contains("shell_integration_enabled"));
    }
}
