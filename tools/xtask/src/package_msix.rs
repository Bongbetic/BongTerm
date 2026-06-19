//! MSIX package staging and validation.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

pub fn run() -> Result<()> {
    let manifest = Path::new("packaging/msix/AppxManifest.xml");
    if !manifest.exists() {
        bail!("missing packaging/msix/AppxManifest.xml");
    }

    let build_status = Command::new("cargo")
        .args(["build", "-p", "bongterm-app", "--release"])
        .status()
        .context("build bongterm-app release binary")?;
    if !build_status.success() {
        bail!("cargo build -p bongterm-app --release failed");
    }

    let exe = Path::new("target/release/bongterm-app.exe");
    if !exe.exists() {
        bail!(
            "missing {}; release build did not produce the application binary",
            exe.display()
        );
    }

    let out_dir = Path::new("target/msix");
    let stage_dir = out_dir.join("stage");
    if stage_dir.exists() {
        fs::remove_dir_all(&stage_dir).context("clear stale MSIX stage dir")?;
    }
    fs::create_dir_all(&stage_dir).context("create MSIX stage dir")?;
    fs::create_dir_all(stage_dir.join("Assets")).context("create MSIX assets dir")?;
    fs::copy(manifest, stage_dir.join("AppxManifest.xml")).context("stage manifest")?;

    for asset in ["Square44x44Logo.png", "Square150x150Logo.png"] {
        let source = Path::new("packaging/msix/assets").join(asset);
        if !source.exists() {
            bail!("missing MSIX asset {}", source.display());
        }
        fs::copy(&source, stage_dir.join("Assets").join(asset))
            .with_context(|| format!("stage MSIX asset {}", source.display()))?;
    }

    fs::copy(exe, stage_dir.join("BongTerm.exe")).context("stage release executable")?;

    let package = out_dir.join("BongTerm.msix");
    let makeappx = find_tool("makeappx.exe")
        .ok_or_else(|| anyhow!("makeappx.exe not found; install Windows SDK MSIX tooling"))?;
    let status = Command::new(&makeappx)
        .args(["pack", "/d"])
        .arg(&stage_dir)
        .args(["/p"])
        .arg(&package)
        .arg("/o")
        .status()
        .with_context(|| format!("run {}", makeappx.display()))?;
    if !status.success() {
        bail!("makeappx failed");
    }

    if let Ok(thumbprint) = env::var("BONGT_SIGN_THUMBPRINT") {
        let signtool = find_tool("signtool.exe").ok_or_else(|| {
            anyhow!("signtool.exe not found; required when BONGT_SIGN_THUMBPRINT is set")
        })?;
        let timestamp_url = env::var("BONGT_TIMESTAMP_URL")
            .unwrap_or_else(|_| "http://timestamp.digicert.com".to_string());
        let status = Command::new(&signtool)
            .args(["sign", "/fd", "SHA256", "/sha1"])
            .arg(thumbprint.trim())
            .args(["/tr"])
            .arg(&timestamp_url)
            .args(["/td", "SHA256"])
            .arg(&package)
            .status()
            .with_context(|| format!("run {} sign", signtool.display()))?;
        if !status.success() {
            bail!("signtool sign failed");
        }

        let verify = Command::new(&signtool)
            .args(["verify", "/pa", "/v"])
            .arg(&package)
            .status()
            .with_context(|| format!("run {} verify", signtool.display()))?;
        if !verify.success() {
            bail!("signtool verify failed");
        }
        println!("package-msix: signed {}", package.display());
    } else {
        println!(
            "package-msix: wrote unsigned package; set BONGT_SIGN_THUMBPRINT for release signing"
        );
    }

    println!("package-msix: wrote {}", package.display());
    Ok(())
}

fn find_tool(exe: &str) -> Option<PathBuf> {
    find_on_path(exe).or_else(|| find_in_windows_sdk(exe))
}

fn find_on_path(exe: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(exe))
        .find(|candidate| candidate.is_file())
}

fn find_in_windows_sdk(exe: &str) -> Option<PathBuf> {
    let root = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut versions = fs::read_dir(root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    versions.sort();
    versions.reverse();

    let arches = ["x64", "x86", "arm64"];
    versions.into_iter().find_map(|version| {
        arches
            .iter()
            .map(|arch| version.join(arch).join(exe))
            .find(|candidate| candidate.is_file())
    })
}
