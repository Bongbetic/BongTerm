//! Emit deterministic SHA-256 sidecars and a combined checksums.txt.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

pub fn run(dir: &Path) -> Result<()> {
    let entries = artifact_files(dir)?;
    if entries.is_empty() {
        bail!("checksums: no artifact files found in {}", dir.display());
    }

    let mut combined = String::new();
    for rel in entries {
        let path = dir.join(&rel);
        let hash = sha256_file(&path)?;
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        fs::write(
            path.with_extension(format!("{}sha256", extension_prefix(&path))),
            format!("{hash}  {rel_text}\n"),
        )
        .with_context(|| format!("write sidecar for {}", path.display()))?;
        combined.push_str(&format!("{hash}  {rel_text}\n"));
    }

    let checksums_path = dir.join("checksums.txt");
    let mut file = fs::File::create(&checksums_path)
        .with_context(|| format!("create {}", checksums_path.display()))?;
    file.write_all(combined.as_bytes())
        .with_context(|| format!("write {}", checksums_path.display()))?;
    println!("checksums: wrote {}", checksums_path.display());
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn artifact_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(dir).min_depth(1).max_depth(1) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == "checksums.txt" || name == "checksums.txt.sig" || name.ends_with(".sha256") {
            continue;
        }
        entries.push(path.strip_prefix(dir)?.to_path_buf());
    }
    entries.sort();
    Ok(entries)
}

fn extension_prefix(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!("{ext}."))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_emit_and_match() {
        let dir = temp_dir("checksums_emit_and_match");
        fs::write(dir.join("BongTerm-0.1.0-mvp0-x64.msix"), b"msix").unwrap();
        fs::write(dir.join("INSTALL.md"), b"install").unwrap();

        run(&dir).unwrap();

        let checksums = fs::read_to_string(dir.join("checksums.txt")).unwrap();
        assert!(checksums.contains("BongTerm-0.1.0-mvp0-x64.msix"));
        assert!(checksums.contains("INSTALL.md"));
        assert!(dir.join("BongTerm-0.1.0-mvp0-x64.msix.sha256").exists());
        assert!(dir.join("INSTALL.md.sha256").exists());
        for line in checksums.lines() {
            let (hash, rel) = line.split_once("  ").unwrap();
            assert_eq!(hash, sha256_file(&dir.join(rel)).unwrap());
        }
        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("bongterm_xtask_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
