//! Gate #28 — `settings_migration_and_last_known_good`
//!
//! Design spec §6.1 criterion #28: "Settings/profile/keybindings **load** +
//! **validation failure** + **backup** + **Safe Mode fallback** work."
//!
//! These are end-to-end tests against the *real* recovery code in
//! [`bongterm_settings`]: [`FileSettingsProvider::load_with_recovery_reporting`],
//! [`bongterm_settings::migrate`], and the on-disk backup writer. Every
//! assertion drives a real load/backup/migrate path:
//!
//! - the backup assertion reads the actual `.corrupt.bak` file off disk and
//!   byte-compares it to the original corrupt bytes;
//! - the migration assertion proves a user-set field survives the version bump
//!   (so `migrate()` cannot be a defaults-stub);
//! - the Safe Mode assertion proves defaults are substituted *and* flagged, and
//!   that the original bad file is left byte-for-byte intact.

use bongterm_settings::{
    CURRENT_SCHEMA_VERSION, FileSettingsProvider, Settings, SettingsProvider, migrate,
};

// Premise of the migration leg of this gate: there must be a prior schema
// version for `migrate()` to step *from*. If the current version were 1 there
// would be no real migration arm to exercise, and the migration assertions
// below would be vacuous. Enforced at compile time.
const _: () = assert!(
    CURRENT_SCHEMA_VERSION > 1,
    "gate #28 migration requires a prior schema version to migrate from"
);

/// Single gate test bundling the four §6.1 #28 behaviors. The required token
/// `settings_migration_and_last_known_good` is the test name itself.
#[test]
fn settings_migration_and_last_known_good() {
    load_valid_file_is_not_safe_mode();
    missing_file_is_defaults_not_safe_mode();
    corrupt_file_triggers_backup_and_safe_mode();
    second_corrupt_load_does_not_clobber_first_backup();
    older_schema_version_is_migrated_and_user_fields_survive();
    last_known_good_snapshot_survives_reload_failure();
}

/// **load valid** — a well-formed JSON5 settings file loads, `safe_mode == false`,
/// no backup is taken, and the parsed values match what was written.
fn load_valid_file_is_not_safe_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json5");
    std::fs::write(
        &path,
        r#"{
          schema_version: 2,
          keybindings: { command_palette: "Ctrl+Alt+P" },
          theme: { name: "high-contrast", font_family: "Cascadia Mono", font_size: 15 },
        }"#,
    )
    .unwrap();

    let (provider, outcome) = FileSettingsProvider::load_with_recovery_reporting(path);

    assert!(
        !outcome.safe_mode,
        "a valid file must not trigger Safe Mode"
    );
    assert!(
        outcome.backup_path.is_none(),
        "a valid file must not produce a backup"
    );
    assert_eq!(
        outcome.settings.keybindings.command_palette, "Ctrl+Alt+P",
        "user keybinding must be loaded verbatim"
    );
    assert_eq!(outcome.settings.theme.name, "high-contrast");
    // The live provider snapshot must reflect the same loaded settings.
    assert_eq!(provider.current().keybindings.command_palette, "Ctrl+Alt+P");
}

/// **missing file is not Safe Mode** — an absent config is not an error: it
/// yields normal defaults with `safe_mode == false` and no backup. This guards
/// the explicit requirement that only a PRESENT-but-broken file triggers Safe
/// Mode.
fn missing_file_is_defaults_not_safe_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does_not_exist.json5");
    assert!(!path.exists());

    let (_provider, outcome) = FileSettingsProvider::load_with_recovery_reporting(path.clone());

    assert!(
        !outcome.safe_mode,
        "a missing file must NOT trigger Safe Mode"
    );
    assert!(
        outcome.backup_path.is_none(),
        "a missing file must not produce a backup"
    );
    assert_eq!(
        outcome.settings,
        Settings::default(),
        "a missing file must load normal defaults"
    );
    // And it must not have created the file as a side effect.
    assert!(!path.exists(), "load must not create the missing file");
}

/// **validation failure → backup + Safe Mode** — a corrupt file is backed up
/// byte-for-byte to a sibling `.corrupt.bak`, defaults are loaded, `safe_mode`
/// is flagged, and the original file is left untouched.
fn corrupt_file_triggers_backup_and_safe_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json5");

    let corrupt_bytes = b"{ this is not valid json5 :::";
    std::fs::write(&path, corrupt_bytes).unwrap();

    let (provider, outcome) = FileSettingsProvider::load_with_recovery_reporting(path.clone());

    // (i) returned settings equal Settings::default()
    assert_eq!(
        outcome.settings,
        Settings::default(),
        "Safe Mode must substitute Settings::default()"
    );
    assert_eq!(
        provider.current().keybindings.command_palette,
        Settings::default().keybindings.command_palette,
        "live provider snapshot must also be defaults in Safe Mode"
    );

    // (ii) safe_mode == true
    assert!(
        outcome.safe_mode,
        "a present-but-broken file must trigger Safe Mode"
    );

    // (iii) the backup file exists on disk and contains the ORIGINAL corrupt
    // bytes — read it back off disk and byte-compare.
    let backup_path = outcome
        .backup_path
        .expect("Safe Mode must record a backup path");
    assert!(
        backup_path.exists(),
        "backup file must exist on disk at {}",
        backup_path.display()
    );
    let backup_bytes = std::fs::read(&backup_path).unwrap();
    assert_eq!(
        backup_bytes, corrupt_bytes,
        "backup must contain the exact original corrupt bytes"
    );
    // Documented naming scheme: <original>.corrupt.bak for the first backup.
    assert_eq!(
        backup_path.file_name().unwrap().to_string_lossy(),
        "settings.json5.corrupt.bak",
        "first backup must use the .corrupt.bak suffix"
    );

    // (iv) the original path was not left half-written — it is byte-identical
    // to what we wrote, never overwritten in place.
    let original_now = std::fs::read(&path).unwrap();
    assert_eq!(
        original_now, corrupt_bytes,
        "the original bad file must be left byte-for-byte intact"
    );
}

/// **no-clobber backup** — when a `.corrupt.bak` already exists, a second
/// corrupt load must NOT overwrite it; it must deterministically pick the next
/// numbered suffix (`.corrupt.1.bak`). Guards the "do not clobber silently,
/// deterministic and testable" requirement and exercises the numbered-suffix
/// path in `next_backup_path`.
fn second_corrupt_load_does_not_clobber_first_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json5");
    let first_bytes = b"{ first corrupt :::";
    let second_bytes = b"{ second DIFFERENT corrupt ???";

    // First corrupt load → creates <path>.corrupt.bak with first_bytes.
    std::fs::write(&path, first_bytes).unwrap();
    let (_p1, out1) = FileSettingsProvider::load_with_recovery_reporting(path.clone());
    let first_backup = out1.backup_path.expect("first load must back up");
    assert_eq!(
        first_backup.file_name().unwrap().to_string_lossy(),
        "settings.json5.corrupt.bak"
    );

    // Second corrupt load with DIFFERENT bytes → must not clobber the first
    // backup; must use the next free numbered suffix.
    std::fs::write(&path, second_bytes).unwrap();
    let (_p2, out2) = FileSettingsProvider::load_with_recovery_reporting(path.clone());
    assert!(out2.safe_mode, "second corrupt load is still Safe Mode");
    let second_backup = out2.backup_path.expect("second load must back up");
    assert_eq!(
        second_backup.file_name().unwrap().to_string_lossy(),
        "settings.json5.corrupt.1.bak",
        "second backup must use the next numbered suffix, not clobber"
    );
    assert_ne!(
        first_backup, second_backup,
        "the two backups must be distinct paths"
    );

    // The first backup must STILL contain the first bytes (untouched), and the
    // second backup must contain the second bytes — both read off disk.
    assert_eq!(
        std::fs::read(&first_backup).unwrap(),
        first_bytes,
        "the first backup must not be clobbered by the second load"
    );
    assert_eq!(
        std::fs::read(&second_backup).unwrap(),
        second_bytes,
        "the second backup must contain the second corrupt bytes"
    );
}

/// **migration** — a document at an OLDER `schema_version` with a real user
/// value loads, ends up at `CURRENT_SCHEMA_VERSION`, and the user value
/// survives. Exercised both directly through `migrate()` and end-to-end through
/// the file loader.
fn older_schema_version_is_migrated_and_user_fields_survive() {
    // --- direct migrate() path ---
    // An explicit older version (1) with a user-set keybinding. Explicit-1
    // proves the `match from_version` v1->v2 arm ran (not "absent → defaulted").
    let old_doc = serde_json::json!({
        "schema_version": 1,
        "keybindings": { "command_palette": "Ctrl+Shift+Q" },
        "theme": { "name": "solarized" }
    });
    let migrated = migrate(old_doc, 1).expect("migration from v1 must succeed");
    assert_eq!(
        migrated.schema_version, CURRENT_SCHEMA_VERSION,
        "migration must bump to the current schema version"
    );
    assert_eq!(
        migrated.keybindings.command_palette, "Ctrl+Shift+Q",
        "user keybinding must survive migration (migrate() is not a defaults-stub)"
    );
    assert_eq!(
        migrated.theme.name, "solarized",
        "user theme name must survive migration"
    );
    // A field the user did NOT set must come through as its default, proving the
    // migrated document is a real, complete Settings.
    assert_eq!(
        migrated.theme.font_family,
        Settings::default().theme.font_family,
        "unset fields must be filled with defaults after migration"
    );

    // --- end-to-end file path: an on-disk legacy doc with NO schema_version
    // (the real shape of pre-v2 configs) must also migrate and preserve fields.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json5");
    std::fs::write(
        &path,
        r#"{ keybindings: { command_palette: "Ctrl+Shift+Q" }, theme: { name: "solarized" } }"#,
    )
    .unwrap();

    let (provider, outcome) = FileSettingsProvider::load_with_recovery_reporting(path);
    assert!(
        !outcome.safe_mode,
        "a migratable legacy file is a success, not Safe Mode"
    );
    assert!(
        outcome.backup_path.is_none(),
        "successful migration must not back up the file"
    );
    assert_eq!(
        outcome.settings.schema_version, CURRENT_SCHEMA_VERSION,
        "loaded legacy file must be migrated to the current version"
    );
    assert_eq!(
        provider.current().keybindings.command_palette,
        "Ctrl+Shift+Q",
        "user value must survive load+migration end-to-end"
    );
    assert_eq!(provider.current().theme.name, "solarized");
}

/// **last-known-good** — preserve the existing reload behavior at the gate
/// level: after a valid load, a reload from a now-corrupt file must FAIL and
/// the provider must keep serving the last valid snapshot. Mirrors the
/// `file_provider_keeps_last_valid_snapshot_after_reload_failure` unit test.
fn last_known_good_snapshot_survives_reload_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json5");
    std::fs::write(
        &path,
        r#"{ keybindings: { command_palette: "Ctrl+Alt+P", new_tab: "Ctrl+Shift+T", close_pane: "Ctrl+Shift+W" }, theme: { name: "dark", font_family: "Cascadia Mono", font_size: 13 } }"#,
    )
    .unwrap();

    let provider = FileSettingsProvider::load_or_default(path.clone()).unwrap();
    assert_eq!(provider.current().keybindings.command_palette, "Ctrl+Alt+P");

    // Corrupt the file, then reload — the reload must error and the in-memory
    // snapshot must be unchanged (last-known-good).
    std::fs::write(&path, "{ keybindings: ").unwrap();
    let err = provider.reload_from_disk().unwrap_err();

    assert!(
        err.to_string().contains("settings reload failed"),
        "reload failure must surface a parse error, got: {err}"
    );
    assert_eq!(
        provider.current().keybindings.command_palette,
        "Ctrl+Alt+P",
        "provider must keep serving the last valid snapshot after a reload failure"
    );
}
