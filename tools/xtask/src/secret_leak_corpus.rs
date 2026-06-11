//! Run synthetic secret corpus through production redactor.

use std::path::PathBuf;

use anyhow::{Context, Result};
use bongterm_security::redactor::Redactor;

#[derive(Debug, serde::Deserialize)]
struct CorpusCase {
    kind: String,
    sample: String,
    must_be_redacted: bool,
}

#[derive(Debug, Default)]
pub struct CorpusReport {
    pub checked: usize,
    pub leaks: usize,
    pub leaked_kinds: Vec<String>,
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/secrets/corpus.jsonl")
}

pub fn run_corpus() -> Result<CorpusReport> {
    let path = corpus_path();
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading corpus at {}", path.display()))?;
    let redactor = Redactor::new();
    let mut report = CorpusReport::default();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let case: CorpusCase =
            serde_json::from_str(line).with_context(|| format!("parsing corpus line: {line}"))?;
        report.checked += 1;
        let redacted = redactor.redact(&format!("value: {} end", case.sample));
        let survived = redacted.contains(&case.sample);
        if case.must_be_redacted && survived {
            report.leaks += 1;
            report.leaked_kinds.push(case.kind);
        }
    }
    Ok(report)
}

pub fn run() -> Result<()> {
    let report = run_corpus()?;
    if report.leaks > 0 {
        anyhow::bail!(
            "secret-leak corpus FAILED: {} leak(s) in kinds {:?} ({} checked)",
            report.leaks,
            report.leaked_kinds,
            report.checked
        );
    }
    println!(
        "secret-leak corpus PASSED: {} cases, 0 leaks",
        report.checked
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_run_has_zero_leaks() {
        let report = run_corpus().expect("corpus run must succeed");
        assert_eq!(
            report.leaks, 0,
            "secret-leak corpus regressions: {:?}",
            report.leaked_kinds
        );
        assert!(
            report.checked >= 7,
            "expected the full corpus to be checked"
        );
    }
}
