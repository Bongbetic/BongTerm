//! Emit local SLSA-style provenance attestation.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

pub fn run(subject: &Path, out: &Path) -> Result<()> {
    run_for_subject(subject, out)
}

pub fn run_for_subject(subject: &Path, out: &Path) -> Result<()> {
    let bytes = fs::read(subject).with_context(|| format!("read {}", subject.display()))?;
    let digest = Sha256::digest(&bytes);
    let subject_name = subject_name(subject);
    let statement = json!({
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [{
            "name": subject_name,
            "digest": { "sha256": format!("{digest:x}") }
        }],
        "predicateType": "https://slsa.dev/provenance/v1",
        "predicate": {
            "buildType": "local-cargo-workspace",
            "builder": { "id": "bongterm-xtask" }
        }
    });
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(out, format!("{}\n", serde_json::to_string(&statement)?))
        .with_context(|| format!("write {}", out.display()))?;
    println!("attestation: wrote {}", out.display());
    Ok(())
}

fn subject_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| path.display().to_string(), ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attestation_references_requested_release_subject_digest() {
        let dir = std::env::temp_dir().join(format!(
            "bongterm_attestation_subject_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let subject = dir.join("BongTerm-0.1.0-mvp0-x64.msix");
        let out = dir.join("attestation.intoto.jsonl");
        fs::write(&subject, b"signed msix bytes").unwrap();

        run_for_subject(&subject, &out).unwrap();

        let text = fs::read_to_string(out).unwrap();
        let digest = format!("{:x}", Sha256::digest(b"signed msix bytes"));
        assert!(text.contains("BongTerm-0.1.0-mvp0-x64.msix"));
        assert!(text.contains(&digest));
        fs::remove_dir_all(dir).unwrap();
    }
}
