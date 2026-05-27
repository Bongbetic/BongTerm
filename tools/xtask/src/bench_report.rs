//! `cargo xtask bench-report [--gate]` — run criterion benches in release mode,
//! produce a Markdown report. Production threshold gating arrives in Phase 5.

use anyhow::{Context, Result};
use std::process::Command;

pub fn run(gate: bool) -> Result<()> {
    let status = Command::new("cargo")
        .args(["bench", "--workspace", "--no-fail-fast"])
        .status()
        .context("cargo bench")?;
    if !status.success() {
        return Err(anyhow::anyhow!("cargo bench failed"));
    }
    let report = "# Benchmark Report\n\nSee `target/criterion/report/index.html` for full details.\n\nPhase 0 baseline: `WezTermAdapter::ingest_bytes` is a scaffold stub.\nReal parser numbers land after Phase 1.B.3.\n";
    std::fs::write("benchmark-report.md", report).context("write benchmark-report.md")?;
    println!("bench-report: wrote benchmark-report.md");
    if gate {
        println!("--gate: production threshold enforcement arrives in Phase 5.");
    }
    Ok(())
}
