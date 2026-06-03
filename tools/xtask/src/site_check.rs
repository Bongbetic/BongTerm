//! Static landing page release-claim checker.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

const REQUIRED_CLAIMS: &[&str] = &[
    "resource-governed agent terminal",
    "no Electron in the terminal hot path",
    "no cloud account required",
    "child-process resource dashboard",
    "Cmd-K command generation",
    "failed-command explanation",
    "Claude and Codex support without bundling",
    "privacy and local-first defaults",
    "experimental MVP-0",
];

pub fn run(dir: &Path) -> Result<()> {
    let index_path = dir.join("index.html");
    let html = fs::read_to_string(&index_path)
        .with_context(|| format!("read {}", index_path.display()))?;
    if !html.contains("<html") || !html.contains("</html>") {
        bail!("site-check: index.html does not look like HTML");
    }
    let missing: Vec<_> = REQUIRED_CLAIMS
        .iter()
        .filter(|claim| !html.contains(**claim))
        .copied()
        .collect();
    if !missing.is_empty() {
        bail!(
            "site-check: missing required claims: {}",
            missing.join(", ")
        );
    }
    verify_internal_links(dir, &html)?;
    println!("site-check: ok ({})", dir.display());
    Ok(())
}

fn verify_internal_links(dir: &Path, html: &str) -> Result<()> {
    for part in html.split("href=\"").skip(1) {
        let Some((href, _)) = part.split_once('"') else {
            bail!("site-check: malformed href");
        };
        if href.starts_with("http") || href.starts_with('#') || href.starts_with("mailto:") {
            continue;
        }
        if !dir.join(href).exists() {
            bail!("site-check: dead internal link: {href}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn site_check_requires_value_prop_claims() {
        let dir = temp_dir("site_check_requires_value_prop_claims");
        fs::write(dir.join("install.html"), "<html></html>").unwrap();
        fs::write(
            dir.join("index.html"),
            r#"<html><body>
resource-governed agent terminal
no Electron in the terminal hot path
no cloud account required
child-process resource dashboard
Cmd-K command generation
failed-command explanation
Claude and Codex support without bundling
privacy and local-first defaults
experimental MVP-0
<a href="install.html">Install</a>
</body></html>"#,
        )
        .unwrap();
        run(&dir).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("bongterm_xtask_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
