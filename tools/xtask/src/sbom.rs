//! Emit a minimal CycloneDX-shaped JSON SBOM (`sbom.cdx.json`).
//! Production-grade tooling decision in Phase 5.

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use serde_json::json;
use std::fs;

pub fn run() -> Result<()> {
    let metadata = MetadataCommand::new().exec().context("cargo metadata")?;
    let mut components = vec![];
    for pkg in &metadata.packages {
        components.push(json!({
            "type": "library",
            "name": pkg.name,
            "version": pkg.version.to_string(),
            "licenses": [{ "license": { "name": pkg.license.clone().unwrap_or_else(|| "UNKNOWN".into()) }}],
            "purl": format!("pkg:cargo/{}@{}", pkg.name, pkg.version),
        }));
    }
    let sbom = json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "tools": [{ "vendor": "BongTerm", "name": "xtask", "version": env!("CARGO_PKG_VERSION") }]
        },
        "components": components,
    });
    fs::write("sbom.cdx.json", serde_json::to_string_pretty(&sbom)?).context("write sbom.cdx.json")?;
    println!("sbom: wrote sbom.cdx.json ({} components)", metadata.packages.len());
    Ok(())
}
