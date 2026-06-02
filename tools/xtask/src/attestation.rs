//! Emit local SLSA-style provenance attestation.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

pub fn run() -> Result<()> {
    let cargo_lock = fs::read("Cargo.lock").context("read Cargo.lock")?;
    let digest = Sha256::digest(&cargo_lock);
    let statement = json!({
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [{
            "name": "Cargo.lock",
            "digest": { "sha256": format!("{digest:x}") }
        }],
        "predicateType": "https://slsa.dev/provenance/v1",
        "predicate": {
            "buildType": "local-cargo-workspace",
            "builder": { "id": "bongterm-xtask" }
        }
    });
    let path = Path::new("attestation.intoto.jsonl");
    fs::write(path, format!("{}\n", serde_json::to_string(&statement)?))
        .context("write attestation.intoto.jsonl")?;
    println!("attestation: wrote {}", path.display());
    Ok(())
}
