//! `cargo xtask check-deps` — validate workspace inter-crate dependency graph
//! against `tools/xtask/allowed-deps.toml`.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct AllowedEntry {
    allowed: Vec<String>,
}

pub fn run() -> Result<()> {
    let manifest = std::fs::read_to_string("tools/xtask/allowed-deps.toml")
        .context("read allowed-deps.toml")?;
    let allowed: BTreeMap<String, AllowedEntry> = toml::from_str(&manifest)
        .context("parse allowed-deps.toml")?;

    let crates_dir = Path::new("crates");
    let mut violations = vec![];

    for entry in std::fs::read_dir(crates_dir).context("read crates/")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("bongterm-") {
            continue;
        }

        let cargo_toml = entry.path().join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }

        let contents = std::fs::read_to_string(&cargo_toml)
            .with_context(|| format!("read {cargo_toml:?}"))?;
        let parsed: toml::Value = toml::from_str(&contents)
            .with_context(|| format!("parse {cargo_toml:?}"))?;

        let mut actual_deps: Vec<String> = vec![];
        for table_key in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(table) = parsed.get(table_key).and_then(|v| v.as_table()) {
                for k in table.keys() {
                    if k.starts_with("bongterm-") {
                        actual_deps.push(k.clone());
                    }
                }
            }
        }

        let allowed_list = allowed
            .get(&name)
            .ok_or_else(|| anyhow!("crate {name} is missing from allowed-deps.toml"))?;

        for dep in &actual_deps {
            if !allowed_list.allowed.contains(dep) {
                violations.push(format!("{name} -> {dep} (not in allowed-deps.toml)"));
            }
        }
    }

    if violations.is_empty() {
        println!("check-deps: ok");
        Ok(())
    } else {
        for v in &violations {
            println!("VIOLATION: {v}");
        }
        Err(anyhow!("{} dependency violation(s)", violations.len()))
    }
}
