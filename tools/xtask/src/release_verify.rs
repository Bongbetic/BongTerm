//! Verify the local release artifact set before a public release.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use bongterm_security::redactor::Redactor;

use crate::checksums::sha256_file;

const REQUIRED: &[&str] = &[
    "BongTerm-0.1.0-mvp0-x64.msix",
    "BongTerm-0.1.0-mvp0-x64.msix.cer",
    "BongTerm-0.1.0-mvp0-x64.msix.sha256",
    "checksums.txt",
    "checksums.txt.sig",
    "attestation.intoto.jsonl",
    "THIRD_PARTY_NOTICES.md",
    "sbom.cdx.json",
    "benchmark-report.md",
    "CHANGELOG.md",
    "known-issues.md",
    "SECURITY.md",
    "INSTALL.md",
];

pub fn run(dir: &Path) -> Result<()> {
    require_artifacts(dir)?;
    verify_checksums(dir)?;
    verify_signature_marker(dir)?;
    verify_attestation(dir)?;
    verify_sbom(dir)?;
    verify_no_secrets(dir)?;
    println!("release-verify: ok ({})", dir.display());
    Ok(())
}

fn require_artifacts(dir: &Path) -> Result<()> {
    let missing: Vec<_> = REQUIRED
        .iter()
        .filter(|name| !dir.join(name).is_file())
        .copied()
        .collect();
    if !missing.is_empty() {
        bail!("release-verify: missing artifacts: {}", missing.join(", "));
    }
    Ok(())
}

fn verify_checksums(dir: &Path) -> Result<()> {
    let text = fs::read_to_string(dir.join("checksums.txt")).context("read checksums.txt")?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Some((expected, rel)) = line.split_once("  ") else {
            bail!("release-verify: malformed checksum line: {line}");
        };
        let actual = sha256_file(&dir.join(rel))?;
        if expected != actual {
            bail!("release-verify: checksum mismatch for {rel}");
        }
    }

    for required in REQUIRED.iter().filter(|name| name.ends_with(".sha256")) {
        let artifact = required.trim_end_matches(".sha256");
        let expected = fs::read_to_string(dir.join(required))
            .with_context(|| format!("read {required}"))?
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        let actual = sha256_file(&dir.join(artifact))?;
        if expected != actual {
            bail!("release-verify: sidecar checksum mismatch for {artifact}");
        }
    }
    Ok(())
}

fn verify_signature_marker(dir: &Path) -> Result<()> {
    let sig = fs::metadata(dir.join("checksums.txt.sig")).context("read checksums.txt.sig")?;
    if sig.len() == 0 {
        bail!("release-verify: checksums.txt.sig is empty");
    }
    Ok(())
}

fn verify_attestation(dir: &Path) -> Result<()> {
    let text = fs::read_to_string(dir.join("attestation.intoto.jsonl"))
        .context("read attestation.intoto.jsonl")?;
    let msix_hash = sha256_file(&dir.join("BongTerm-0.1.0-mvp0-x64.msix"))?;
    let mut found = false;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line).context("parse attestation")?;
        if value["_type"] != "https://in-toto.io/Statement/v1" {
            bail!("release-verify: attestation _type mismatch");
        }
        if value["predicateType"] != "https://slsa.dev/provenance/v1" {
            bail!("release-verify: attestation predicateType mismatch");
        }
        found |= line.contains("BongTerm-0.1.0-mvp0-x64.msix") && line.contains(&msix_hash);
    }
    if !found {
        bail!("release-verify: attestation does not reference MSIX digest");
    }
    Ok(())
}

fn verify_sbom(dir: &Path) -> Result<()> {
    let text = fs::read_to_string(dir.join("sbom.cdx.json")).context("read sbom.cdx.json")?;
    if !text.contains("vendored-wezterm") {
        bail!("release-verify: sbom.cdx.json missing vendored-wezterm component");
    }
    Ok(())
}

fn verify_no_secrets(dir: &Path) -> Result<()> {
    let redactor = Redactor::new();
    for name in REQUIRED {
        let path = dir.join(name);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let preview = redactor.preview(&text);
        if preview.match_count > 0 {
            bail!("release-verify: possible secret in {name}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksums;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn release_verify_passes_on_good() {
        let dir = fixture("release_verify_passes_on_good");
        make_good_dist(&dir);
        run(&dir).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn release_verify_fails_missing_artifact() {
        let dir = fixture("release_verify_fails_missing_artifact");
        make_good_dist(&dir);
        fs::remove_file(dir.join("INSTALL.md")).unwrap();
        assert!(run(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn release_verify_fails_bad_checksum() {
        let dir = fixture("release_verify_fails_bad_checksum");
        make_good_dist(&dir);
        fs::write(dir.join("BongTerm-0.1.0-mvp0-x64.msix"), b"tampered").unwrap();
        assert!(run(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn release_verify_fails_secret_leak() {
        let dir = fixture("release_verify_fails_secret_leak");
        make_good_dist(&dir);
        fs::write(
            dir.join("known-issues.md"),
            "leaked ghp_1234567890abcdefghijklmnopqrstuvwx",
        )
        .unwrap();
        checksums::run(&dir).unwrap();
        fs::write(dir.join("checksums.txt.sig"), b"sig").unwrap();
        write_attestation(&dir);
        assert!(run(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    fn make_good_dist(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        for name in REQUIRED {
            if *name == "checksums.txt" || name.ends_with(".sha256") {
                continue;
            }
            fs::write(dir.join(name), format!("{name}\n")).unwrap();
        }
        fs::write(
            dir.join("sbom.cdx.json"),
            r#"{"components":[{"name":"vendored-wezterm"}]}"#,
        )
        .unwrap();
        write_attestation(dir);
        checksums::run(dir).unwrap();
        fs::write(dir.join("checksums.txt.sig"), b"sig").unwrap();
        let hash = checksums::sha256_file(&dir.join("BongTerm-0.1.0-mvp0-x64.msix")).unwrap();
        fs::write(
            dir.join("BongTerm-0.1.0-mvp0-x64.msix.sha256"),
            format!("{hash}  BongTerm-0.1.0-mvp0-x64.msix\n"),
        )
        .unwrap();
    }

    fn write_attestation(dir: &Path) {
        let hash = checksums::sha256_file(&dir.join("BongTerm-0.1.0-mvp0-x64.msix")).unwrap();
        let statement = json!({
            "_type": "https://in-toto.io/Statement/v1",
            "subject": [{
                "name": "BongTerm-0.1.0-mvp0-x64.msix",
                "digest": { "sha256": hash }
            }],
            "predicateType": "https://slsa.dev/provenance/v1",
            "predicate": {}
        });
        fs::write(
            dir.join("attestation.intoto.jsonl"),
            format!("{}\n", serde_json::to_string(&statement).unwrap()),
        )
        .unwrap();
    }

    fn fixture(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("bongterm_xtask_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
