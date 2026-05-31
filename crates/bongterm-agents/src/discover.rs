//! Binary discovery: PATH resolution, version probe, auth probe.
//!
//! Discovery is injectable so unit tests never depend on a real CLI being
//! installed. Production callers use [`BinaryDiscovery::probe_real`].

use crate::{AuthState, DiscoveryResult};

/// Resolves an agent CLI binary and reports version + auth.
pub struct BinaryDiscovery {
    binary_name: String,
    located: Option<String>,
}

impl BinaryDiscovery {
    /// Create a discovery that will resolve `binary_name` via PATH.
    #[must_use]
    pub fn new(binary_name: impl Into<String>) -> Self {
        let binary_name = binary_name.into();
        let located = which::which(&binary_name)
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        Self {
            binary_name,
            located,
        }
    }

    /// Create a discovery with an explicit located path (tests).
    #[must_use]
    pub fn with_located(binary_name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            binary_name: binary_name.into(),
            located: Some(path.into()),
        }
    }

    /// Probe using injected version/auth closures (deterministic for tests).
    pub fn probe(
        &self,
        version_of: impl Fn(&str) -> Option<String>,
        auth_of: impl Fn(&str) -> AuthState,
    ) -> DiscoveryResult {
        match &self.located {
            None => DiscoveryResult {
                found: false,
                binary_path: None,
                version: None,
                auth_state: AuthState::Unknown,
            },
            Some(path) => {
                let version = version_of(path).and_then(|line| parse_version_line(&line));
                let auth_state = auth_of(path);
                DiscoveryResult {
                    found: true,
                    binary_path: Some(path.clone()),
                    version,
                    auth_state,
                }
            }
        }
    }

    /// Production probe: runs `<binary> --version` and inspects an auth marker.
    #[must_use]
    pub fn probe_real(&self, auth_env: &str) -> DiscoveryResult {
        self.probe(
            |path| {
                std::process::Command::new(path)
                    .arg("--version")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            },
            |_path| {
                if std::env::var(auth_env).is_ok() {
                    AuthState::Authenticated
                } else {
                    AuthState::Unauthenticated
                }
            },
        )
    }

    /// The binary name this discovery resolves.
    #[must_use]
    pub fn binary_name(&self) -> &str {
        &self.binary_name
    }
}

/// Extract the first dotted `MAJOR.MINOR.PATCH` token from a version line.
#[must_use]
pub fn parse_version_line(line: &str) -> Option<String> {
    for token in line.split([' ', '(', ')']) {
        let t = token.trim_start_matches('v');
        let dots = t.bytes().filter(|&b| b == b'.').count();
        if dots == 2 && !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Some(t.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_yields_not_found() {
        let d = BinaryDiscovery::new("definitely-not-a-real-binary-xyz");
        let r = d.probe(|_| None, |_| AuthState::Unknown);
        assert!(!r.found);
        assert_eq!(r.auth_state, AuthState::Unknown);
        assert!(r.binary_path.is_none());
    }

    #[test]
    fn version_parser_extracts_semver_token() {
        assert_eq!(
            parse_version_line("claude 1.2.3"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            parse_version_line("codex-cli v0.9.0 (build 7)"),
            Some("0.9.0".to_string())
        );
        assert_eq!(parse_version_line("no version here"), None);
    }

    #[test]
    fn found_binary_uses_injected_version_and_auth() {
        let d = BinaryDiscovery::with_located("claude", "C:\\bin\\claude.exe");
        let r = d.probe(
            |_| Some("claude 9.9.9".to_string()),
            |_| AuthState::Authenticated,
        );
        assert!(r.found);
        assert_eq!(r.version.as_deref(), Some("9.9.9"));
        assert_eq!(r.auth_state, AuthState::Authenticated);
        assert_eq!(r.binary_path.as_deref(), Some("C:\\bin\\claude.exe"));
    }
}
