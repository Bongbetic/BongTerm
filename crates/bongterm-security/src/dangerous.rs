//! Dangerous-command pattern matcher.

use crate::EnforcementLevel;

/// Classified dangerous-command kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DangerKind {
    GitForcePush,
    RecursiveDelete,
    KubectlDelete,
    TerraformDestroy,
}

impl DangerKind {
    #[must_use]
    pub fn enforcement(self) -> EnforcementLevel {
        match self {
            Self::GitForcePush
            | Self::RecursiveDelete
            | Self::KubectlDelete
            | Self::TerraformDestroy => EnforcementLevel::RequireApproval,
        }
    }
}

/// Matches command lines against known destructive patterns.
pub struct DangerousCommandMatcher;

impl Default for DangerousCommandMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DangerousCommandMatcher {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[allow(clippy::unused_self)]
    #[must_use]
    pub fn classify(&self, command: &str) -> Option<DangerKind> {
        let command = command.to_lowercase();
        let tokens: Vec<&str> = command.split_whitespace().collect();
        let has = |word: &str| tokens.contains(&word);

        if has("git") && has("push") && tokens.iter().any(|token| token.starts_with("--force")) {
            return Some(DangerKind::GitForcePush);
        }
        if has("rm")
            && (tokens
                .iter()
                .any(|token| *token == "-rf" || *token == "-fr")
                || (tokens.contains(&"-r") && tokens.contains(&"-f")))
        {
            return Some(DangerKind::RecursiveDelete);
        }
        if has("kubectl") && has("delete") {
            return Some(DangerKind::KubectlDelete);
        }
        if has("terraform") && has("destroy") {
            return Some(DangerKind::TerraformDestroy);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_known_dangerous_commands() {
        let matcher = DangerousCommandMatcher::new();
        assert_eq!(
            matcher.classify("git push --force origin main"),
            Some(DangerKind::GitForcePush)
        );
        assert_eq!(
            matcher.classify("git push --force-with-lease"),
            Some(DangerKind::GitForcePush)
        );
        assert_eq!(
            matcher.classify("rm -rf /"),
            Some(DangerKind::RecursiveDelete)
        );
        assert_eq!(
            matcher.classify("sudo rm -rf /var"),
            Some(DangerKind::RecursiveDelete)
        );
        assert_eq!(
            matcher.classify("kubectl delete pod foo"),
            Some(DangerKind::KubectlDelete)
        );
        assert_eq!(
            matcher.classify("terraform destroy -auto-approve"),
            Some(DangerKind::TerraformDestroy)
        );
    }

    #[test]
    fn benign_commands_are_not_flagged() {
        let matcher = DangerousCommandMatcher::new();
        assert_eq!(matcher.classify("git push origin main"), None);
        assert_eq!(matcher.classify("ls -la"), None);
        assert_eq!(matcher.classify("rm file.txt"), None);
        assert_eq!(matcher.classify("kubectl get pods"), None);
        assert_eq!(matcher.classify("terraform plan"), None);
    }

    #[test]
    fn dangerous_command_requires_approval_not_advisory() {
        let matcher = DangerousCommandMatcher::new();
        let kind = matcher.classify("rm -rf /").unwrap();
        assert_eq!(kind.enforcement(), EnforcementLevel::RequireApproval);
    }
}
