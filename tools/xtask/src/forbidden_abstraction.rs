//! Static check for forbidden implementation techniques.

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

const NEEDLES: &[&str] = &[
    "CreateRemoteThread",
    "WriteProcessMemory",
    "ReadConsoleOutput",
    "NtCreate",
    "SetWindowsHookEx",
    "process hollow",
    "kernel-driver",
];

const ALLOWLIST: &[&str] = &[
    "crates\\bongterm-security\\src\\forbidden.rs",
    "crates\\bongterm-test-kit\\src\\conformance\\process_tree_auditor_conformance.rs",
    "tools\\xtask\\src\\forbidden_abstraction.rs",
];

pub fn run() -> Result<()> {
    let mut hits = Vec::new();
    for entry in WalkDir::new("crates")
        .into_iter()
        .chain(WalkDir::new("tools"))
    {
        let entry = entry.context("walk source tree")?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let display = path.to_string_lossy().replace('/', "\\");
        if ALLOWLIST.iter().any(|allowed| display.ends_with(allowed)) {
            continue;
        }
        let contents = std::fs::read_to_string(path).context("read source file")?;
        for needle in NEEDLES {
            if contents.contains(needle) {
                hits.push(format!("{}: {}", path.display(), needle));
            }
        }
    }

    if !hits.is_empty() {
        bail!("forbidden abstraction markers found:\n{}", hits.join("\n"));
    }
    println!("forbidden-abstraction: passed");
    Ok(())
}
