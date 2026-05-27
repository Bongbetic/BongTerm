//! `cargo xtask upstream-sync` — emit Markdown changelog of vendor/wezterm
//! delta between the currently checked-out submodule commit and a target tag.
//!
//! Usage:
//!   cargo xtask upstream-sync                  # delta vs HEAD
//!   BONGT_WEZTERM_TARGET=<tag> cargo xtask upstream-sync

use anyhow::{Context, Result};
use std::process::Command;

pub fn run() -> Result<()> {
    let target = std::env::var("BONGT_WEZTERM_TARGET").unwrap_or_else(|_| "HEAD".to_string());

    let current = run_git_in_submodule(&["rev-parse", "HEAD"])?;
    let log = run_git_in_submodule(&[
        "log",
        "--oneline",
        &format!("{}..{}", current.trim(), target),
    ])?;

    println!("# WezTerm upstream-sync report\n");
    println!("Current pin: `{}`", current.trim());
    println!("Target:      `{}`\n", target);
    println!("## Commits in target not in current pin\n");
    if log.trim().is_empty() {
        println!("_(none — pin is up to date)_");
    } else {
        for line in log.lines() {
            println!("- {line}");
        }
    }
    Ok(())
}

fn run_git_in_submodule(args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir("vendor/wezterm")
        .output()
        .with_context(|| format!("git {} in vendor/wezterm", args.join(" ")))?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "git failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
