//! MSIX package staging and validation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

pub fn run() -> Result<()> {
    let manifest = Path::new("packaging/msix/AppxManifest.xml");
    if !manifest.exists() {
        bail!("missing packaging/msix/AppxManifest.xml");
    }

    let out_dir = Path::new("target/msix");
    let stage_dir = out_dir.join("stage");
    fs::create_dir_all(&stage_dir).context("create MSIX stage dir")?;
    fs::copy(manifest, stage_dir.join("AppxManifest.xml")).context("stage manifest")?;

    let exe = Path::new("target/release/bongterm-app.exe");
    let payload = stage_dir.join("BongTerm.exe");
    if exe.exists() {
        fs::copy(exe, &payload).context("stage release executable")?;
    } else {
        fs::write(&payload, b"bongterm release payload placeholder")
            .context("stage placeholder payload")?;
    }

    let package = out_dir.join("BongTerm.msix");
    if let Some(makeappx) = find_on_path("makeappx.exe") {
        let status = std::process::Command::new(makeappx)
            .args(["pack", "/d"])
            .arg(&stage_dir)
            .args(["/p"])
            .arg(&package)
            .status()
            .context("run makeappx")?;
        if !status.success() {
            bail!("makeappx failed");
        }
    } else {
        fs::write(
            &package,
            b"MSIX placeholder; install smoke requires Windows SDK makeappx",
        )
        .context("write placeholder MSIX")?;
    }

    println!("package-msix: wrote {}", package.display());
    Ok(())
}

fn find_on_path(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(exe))
        .find(|candidate| candidate.is_file())
}
