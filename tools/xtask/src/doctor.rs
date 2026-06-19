//! `cargo xtask doctor` — environment readiness check.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run() -> Result<()> {
    let mut report = Report::default();

    report.check("Windows version", check_windows_version);
    report.check("Rust toolchain", check_rust_toolchain);
    report.check("Visual Studio Build Tools", check_vs_build_tools);
    report.check("Windows SDK", check_windows_sdk);
    report.check("MSIX tooling", check_msix_tooling);
    report.check("Submodule state", check_submodule_state);
    report.check("Code-signing certificate", check_code_signing_cert);
    report.check("Defender status", check_defender);
    report.check("GPU adapter", check_gpu_adapter);
    report.check("WSL availability", check_wsl);

    report.print();
    if report.has_failures() {
        Err(anyhow!(
            "doctor reports {} failure(s)",
            report.failure_count()
        ))
    } else {
        Ok(())
    }
}

#[derive(Default)]
struct Report {
    rows: Vec<(String, std::result::Result<String, String>)>,
}

impl Report {
    fn check<F>(&mut self, name: &str, f: F)
    where
        F: FnOnce() -> Result<String>,
    {
        let outcome = f().map_err(|e| e.to_string());
        self.rows.push((name.to_string(), outcome));
    }

    fn has_failures(&self) -> bool {
        self.rows.iter().any(|(_, r)| r.is_err())
    }

    fn failure_count(&self) -> usize {
        self.rows.iter().filter(|(_, r)| r.is_err()).count()
    }

    fn print(&self) {
        for (name, r) in &self.rows {
            match r {
                Ok(detail) => println!("  ok  {name:30} {detail}"),
                Err(err) => println!(" FAIL {name:30} {err}"),
            }
        }
    }
}

fn check_windows_version() -> Result<String> {
    let out = Command::new("cmd")
        .args(["/c", "ver"])
        .output()
        .context("cmd ver")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn check_rust_toolchain() -> Result<String> {
    let out = Command::new("rustc")
        .arg("--version")
        .output()
        .context("rustc --version")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn check_vs_build_tools() -> Result<String> {
    let cl = find_on_path("cl.exe")
        .or_else(find_visual_studio_cl)
        .ok_or_else(|| anyhow!("cl.exe not found; install VS Build Tools 2022"))?;
    Command::new(&cl)
        .arg("/?")
        .output()
        .with_context(|| format!("run {}", cl.display()))?;
    Ok(cl.display().to_string())
}

fn check_windows_sdk() -> Result<String> {
    let candidates = [r"C:\Program Files (x86)\Windows Kits\10\bin"];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return Ok(format!("Windows Kits found at {c}"));
        }
    }
    Err(anyhow!("Windows SDK not found"))
}

fn check_msix_tooling() -> Result<String> {
    let signtool = find_tool("signtool.exe")
        .ok_or_else(|| anyhow!("signtool.exe not found; install Windows SDK signing tools"))?;
    let makeappx = find_tool("makeappx.exe")
        .ok_or_else(|| anyhow!("makeappx.exe not found; install Windows SDK MSIX tools"))?;
    Ok(format!(
        "signtool={}, makeappx={}",
        signtool.display(),
        makeappx.display()
    ))
}

fn check_submodule_state() -> Result<String> {
    let out = Command::new("git")
        .args(["submodule", "status"])
        .output()
        .context("git submodule status")?;
    let s = String::from_utf8_lossy(&out.stdout);
    if s.lines().any(|l| l.starts_with('+') || l.starts_with('-')) {
        Err(anyhow!("submodule dirty or uninitialized: {}", s.trim()))
    } else {
        Ok(s.trim().to_string())
    }
}

fn check_code_signing_cert() -> Result<String> {
    Ok("not checked (Phase 5 will verify)".to_string())
}

fn check_defender() -> Result<String> {
    let ps = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-MpComputerStatus).RealTimeProtectionEnabled",
        ])
        .output()
        .context("Get-MpComputerStatus")?;
    Ok(String::from_utf8_lossy(&ps.stdout).trim().to_string())
}

fn check_gpu_adapter() -> Result<String> {
    let ps = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
        ])
        .output()
        .context("Win32_VideoController")?;
    Ok(String::from_utf8_lossy(&ps.stdout)
        .trim()
        .replace('\n', " | "))
}

fn check_wsl() -> Result<String> {
    let out = Command::new("wsl")
        .arg("--status")
        .output()
        .context("wsl --status")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn find_tool(exe: &str) -> Option<PathBuf> {
    find_on_path(exe).or_else(|| find_in_windows_sdk(exe))
}

fn find_on_path(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
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

fn find_visual_studio_cl() -> Option<PathBuf> {
    let vswhere =
        Path::new(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if !vswhere.is_file() {
        return None;
    }

    let out = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-find",
            r"VC\Tools\MSVC\**\bin\Hostx64\x64\cl.exe",
        ])
        .output()
        .ok()?;

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| PathBuf::from(line.trim()))
        .find(|path| path.is_file())
}
