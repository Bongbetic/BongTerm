//! BongTerm xtask runner.
//!
//! Subcommands implement the build / lint / SBOM / bench / corpus tasks listed
//! in spec §2.7. Each subcommand lives in its own module; `doctor` and
//! `check_deps` are implemented; others are stubs fleshed out in later phases.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "BongTerm workspace tasks")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Diagnose local environment readiness.
    Doctor,
    /// Verify workspace dependency graph against tools/xtask/allowed-deps.toml.
    CheckDeps,
    /// Ensure every packaged artifact ships THIRD_PARTY_NOTICES.md.
    CheckLicenses,
    /// Generate Markdown changelog of vendor/wezterm delta against pinned tag.
    UpstreamSync,
    /// Generate CycloneDX SBOM from Cargo.lock + vendor/wezterm.
    Sbom,
    /// Run criterion benches and produce release-notes-ready report.
    BenchReport {
        /// Fail on absolute-budget violation.
        #[arg(long)]
        gate: bool,
    },
    /// Run known synthetic token corpus through redaction pipeline.
    SecretLeakCorpus,
    /// Run poisoned content corpus through agent observer + policy.
    PromptInjectionCorpus,
    /// Remove orphaned sidecar chunks.
    CleanupChunks,
    /// Produce signed MSIX artifact (Phase 5).
    PackageMsix,
    /// Emit SLSA provenance attestation.
    Attestation,
    /// Emit SHA-256 sidecars and combined checksums.txt over a dist directory.
    Checksums {
        /// Dist directory to checksum.
        dir: PathBuf,
    },
    /// Verify release artifact completeness, checksums, attestation, SBOM, and secrets.
    ReleaseVerify {
        /// Dist directory to verify.
        dir: PathBuf,
    },
    /// Validate static landing page required claims and internal links.
    SiteCheck {
        /// Site directory to validate.
        dir: PathBuf,
    },
    /// Scan for forbidden implementation techniques.
    ForbiddenAbstraction,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Doctor => doctor::run(),
        Cmd::CheckDeps => check_deps::run(),
        Cmd::CheckLicenses => check_licenses::run(),
        Cmd::UpstreamSync => upstream_sync::run(),
        Cmd::Sbom => sbom::run(),
        Cmd::BenchReport { gate } => bench_report::run(gate),
        Cmd::SecretLeakCorpus => secret_leak_corpus::run(),
        Cmd::PromptInjectionCorpus => prompt_injection_corpus::run(),
        Cmd::CleanupChunks => cleanup_chunks::run(),
        Cmd::PackageMsix => package_msix::run(),
        Cmd::Attestation => attestation::run(),
        Cmd::Checksums { dir } => checksums::run(&dir),
        Cmd::ReleaseVerify { dir } => release_verify::run(&dir),
        Cmd::SiteCheck { dir } => site_check::run(&dir),
        Cmd::ForbiddenAbstraction => forbidden_abstraction::run(),
    }
}

mod attestation;
mod bench_report;
mod check_deps;
mod check_licenses;
mod checksums;
mod cleanup_chunks;
mod doctor;
mod forbidden_abstraction;
mod package_msix;
mod prompt_injection_corpus;
mod release_verify;
mod sbom;
mod secret_leak_corpus;
mod site_check;
mod upstream_sync;
