//! `cargo xtask doctor` — environment readiness check.

use anyhow::{Context, Result, anyhow};
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
    Command::new("cl.exe")
        .arg("/?")
        .output()
        .map_err(|_| anyhow!("cl.exe not on PATH; install VS Build Tools 2022"))?;
    Ok("cl.exe present".to_string())
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
    let signtool = which("signtool.exe")?;
    let makeappx = which("makeappx.exe").ok();
    Ok(match makeappx {
        Some(m) => format!("signtool={signtool}, makeappx={m}"),
        None => format!("signtool={signtool}; makeappx missing (install via Win SDK)"),
    })
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

fn which(exe: &str) -> Result<String> {
    let out = Command::new("where")
        .arg(exe)
        .output()
        .context("where command")?;
    let s = String::from_utf8_lossy(&out.stdout);
    let first = s
        .lines()
        .next()
        .ok_or_else(|| anyhow!("{exe} not on PATH"))?;
    Ok(first.trim().to_string())
}
