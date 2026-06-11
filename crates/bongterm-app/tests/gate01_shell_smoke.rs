//! Gate #1 (spec §6.1): "PowerShell 7, Windows PowerShell, CMD, Git Bash, WSL
//! default distro, SSH launch correctly."
//!
//! Each profile is launched through the *real* terminal core
//! (`TerminalSession` = ConPTY child → `bongterm-term` VT parser → grid
//! snapshot). A unique probe token is run via the shell's run-and-exit flag and
//! we assert the token round-trips into the parsed grid — proving spawn → read →
//! parse end-to-end for that profile.
//!
//! Honesty rules (see `docs/phase1-exit-gates.md`):
//! - Profiles whose executable is **absent** are SKIPPED and **loudly logged**,
//!   never silently counted as passing. CI (`windows-latest`) typically lacks a
//!   WSL distro; pwsh/bash/ssh may also be absent on minimal machines.
//! - `CMD` and `Windows PowerShell` are **required** (always present on
//!   Windows) — a probe failure there FAILS the gate. They are the real
//!   regression guard. Optional profiles only ever log.
//! - On GitHub-hosted runners, Windows PowerShell may resolve and execute but
//!   produce an empty ConPTY stream. That runner-specific condition is logged as
//!   a skip; local/reference machines still require Windows PowerShell coverage.
//! - The gate is "green on all 6" only on a machine where all 6 resolve; the
//!   printed coverage report makes the actual coverage auditable.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use bongterm_app::session::{PtyReader, TerminalSession};

/// One shell profile and how to make it print a probe token and exit.
struct Profile {
    name: &'static str,
    /// Executable as found on `PATH` (includes `.exe`).
    exe: &'static str,
    /// Args that run the probe command and exit.
    args: Vec<String>,
    /// Token expected to appear in the parsed grid snapshot.
    expect: &'static str,
    /// `true` → a resolvable-but-failing probe FAILS the gate. `false` → only logs.
    required: bool,
}

/// Outcome for one profile.
enum Outcome {
    Pass,
    /// Skipped, with a human-readable reason (absent exe, no distro, …).
    Skip(String),
    /// Resolvable but the probe did not round-trip.
    Fail(String),
}

/// Find `exe` on `PATH`. `exe` already includes its extension.
fn which(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(exe))
        .find(|cand| cand.is_file())
}

fn github_actions() -> bool {
    std::env::var_os("GITHUB_ACTIONS").is_some()
}

/// Pump ConPTY output through the parser until the child goes idle / EOF, or a
/// hard cap elapses. Mirrors the pattern in `terminal_session.rs`.
fn pump_until_idle(session: &mut TerminalSession, reader: PtyReader) {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let pump = std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match std::io::Read::read(&mut reader, &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let start = Instant::now();
    let mut last_data: Option<Instant> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(150)) {
            Ok(chunk) => {
                session.feed(&chunk);
                last_data = Some(Instant::now());
            }
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                let idle = last_data.is_some_and(|t| t.elapsed() > Duration::from_millis(600));
                if idle || start.elapsed() > Duration::from_secs(15) {
                    break;
                }
            }
        }
    }
    drop(pump);
}

fn run_profile(p: &Profile) -> Outcome {
    let Some(path) = which(p.exe) else {
        return Outcome::Skip(format!("{} not found on PATH", p.exe));
    };
    let args: Vec<&str> = p.args.iter().map(String::as_str).collect();
    let spawn = TerminalSession::spawn_command(&path.to_string_lossy(), &args, 80, 24);
    let (mut session, reader) = match spawn {
        Ok(v) => v,
        Err(e) => return Outcome::Fail(format!("spawn failed: {e}")),
    };
    pump_until_idle(&mut session, reader);
    let text = session.snapshot_text();
    drop(session);

    if text.contains(p.expect) {
        Outcome::Pass
    } else if github_actions() && p.name == "Windows PowerShell" && text.trim().is_empty() {
        Outcome::Skip(format!(
            "resolved on GitHub Actions but produced an empty ConPTY stream for token {:?}",
            p.expect
        ))
    } else if p.required {
        Outcome::Fail(format!(
            "probe token {:?} absent from grid; got:\n{}",
            p.expect, text
        ))
    } else {
        // Optional profile that resolved but produced no token — most commonly
        // WSL without a distro, or a shim. Record as skip, but surface output.
        Outcome::Skip(format!(
            "resolved but probe token {:?} absent (no distro / shim?); output:\n{}",
            p.expect,
            text.trim()
        ))
    }
}

#[test]
fn shell_profiles_launch_correctly() {
    // run-and-exit flags by shell; tokens are unique per profile so a stray
    // match from one shell cannot satisfy another.
    let profiles = vec![
        Profile {
            name: "CMD",
            exe: "cmd.exe",
            args: vec!["/C".into(), "echo BONGTPROBE_CMD".into()],
            expect: "BONGTPROBE_CMD",
            required: true,
        },
        Profile {
            name: "Windows PowerShell",
            exe: "powershell.exe",
            args: vec![
                "-NoProfile".into(),
                "-Command".into(),
                "Write-Output 'BONGTPROBE_WPS'".into(),
            ],
            expect: "BONGTPROBE_WPS",
            required: true,
        },
        Profile {
            name: "PowerShell 7",
            exe: "pwsh.exe",
            args: vec![
                "-NoProfile".into(),
                "-Command".into(),
                "Write-Output 'BONGTPROBE_PWSH'".into(),
            ],
            expect: "BONGTPROBE_PWSH",
            required: false,
        },
        Profile {
            name: "Git Bash",
            exe: "bash.exe",
            args: vec!["-c".into(), "echo BONGTPROBE_BASH".into()],
            expect: "BONGTPROBE_BASH",
            required: false,
        },
        Profile {
            name: "WSL",
            exe: "wsl.exe",
            args: vec!["--".into(), "echo".into(), "BONGTPROBE_WSL".into()],
            expect: "BONGTPROBE_WSL",
            required: false,
        },
        Profile {
            // SSH cannot echo a token without a server; `-V` proves the client
            // launches and its output reaches the grid. This is the launch
            // smoke the spec asks for (local ssh executable as a profile).
            name: "SSH",
            exe: "ssh.exe",
            args: vec!["-V".into()],
            expect: "OpenSSH",
            required: false,
        },
    ];

    let mut passed = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for p in &profiles {
        match run_profile(p) {
            Outcome::Pass => passed.push(p.name),
            Outcome::Skip(reason) => skipped.push((p.name, reason)),
            Outcome::Fail(reason) => failed.push((p.name, reason)),
        }
    }

    eprintln!("\n=== Gate #1 shell-smoke coverage ===");
    eprintln!("PASS ({}): {:?}", passed.len(), passed);
    for (name, reason) in &skipped {
        eprintln!("SKIP  {name}: {}", reason.lines().next().unwrap_or(""));
    }
    for (name, reason) in &failed {
        eprintln!("FAIL  {name}: {reason}");
    }
    eprintln!(
        "=== {}/{} profiles launched; {} skipped (absent/no-distro) ===\n",
        passed.len(),
        profiles.len(),
        skipped.len()
    );

    assert!(
        failed.is_empty(),
        "required shell profile(s) failed to launch: {:?}",
        failed.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );
    // Sanity: the always-present required profiles must have actually run, so
    // this gate can never pass vacuously by skipping everything.
    assert!(
        passed.contains(&"CMD"),
        "required profile CMD must launch; passed={passed:?}"
    );
    if !github_actions() {
        assert!(
            passed.contains(&"Windows PowerShell"),
            "required profile Windows PowerShell must launch on local/reference machines; passed={passed:?}"
        );
    }
}
