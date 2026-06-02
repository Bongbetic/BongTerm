//! Best-effort secret redactor for persisted/exported/indexed text only.
//! Never mutate raw terminal display.

/// Token kinds recognized by corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    AwsAccessKeyId,
    GitHubPat,
    OpenAiKey,
    AnthropicKey,
    Jwt,
    HighEntropy,
}

const PLACEHOLDER: &str = "[REDACTED]";

/// Preview shown before any opt-in export.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedactionPreview {
    pub original: String,
    pub redacted: String,
    pub match_count: usize,
}

/// Redacts known secret formats from text.
pub struct Redactor;

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn redact(&self, input: &str) -> String {
        let ssh_redacted = self.redact_ssh_blocks(input);
        let mut out = String::with_capacity(ssh_redacted.len());
        for token in ssh_redacted.split_inclusive(char::is_whitespace) {
            let (core, trailing) = split_trailing_ws(token);
            if core != PLACEHOLDER
                && let Some((start, end)) = sensitive_range(core)
            {
                out.push_str(&core[..start]);
                out.push_str(PLACEHOLDER);
                out.push_str(&core[end..]);
                out.push_str(trailing);
                continue;
            }
            out.push_str(token);
        }
        out
    }

    #[must_use]
    pub fn preview(&self, bundle: &str) -> RedactionPreview {
        let redacted = self.redact(bundle);
        let match_count = redacted.matches(PLACEHOLDER).count();
        RedactionPreview {
            original: bundle.to_string(),
            redacted,
            match_count,
        }
    }

    #[allow(clippy::unused_self)]
    fn redact_ssh_blocks(&self, input: &str) -> String {
        let mut redacting = false;
        let mut lines = Vec::new();
        for line in input.lines() {
            if line.contains("BEGIN") && line.contains("PRIVATE KEY") {
                redacting = true;
                lines.push(line.to_string());
                continue;
            }
            if line.contains("END") && line.contains("PRIVATE KEY") {
                redacting = false;
                lines.push(line.to_string());
                continue;
            }
            if redacting && !line.trim().is_empty() {
                lines.push(PLACEHOLDER.to_string());
            } else {
                lines.push(line.to_string());
            }
        }
        let joined = lines.join("\n");
        if input.ends_with('\n') {
            format!("{joined}\n")
        } else {
            joined
        }
    }

    #[must_use]
    pub fn classify(token: &str) -> Option<TokenKind> {
        if token.starts_with("AKIA")
            && token.len() == 20
            && token[4..]
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Some(TokenKind::AwsAccessKeyId);
        }
        if token.starts_with("ghp_") && token.len() >= 36 {
            return Some(TokenKind::GitHubPat);
        }
        if token.starts_with("sk-ant-") {
            return Some(TokenKind::AnthropicKey);
        }
        if token.starts_with("sk-") && token.len() >= 20 {
            return Some(TokenKind::OpenAiKey);
        }
        if is_jwt(token) {
            return Some(TokenKind::Jwt);
        }
        if is_high_entropy(token) {
            return Some(TokenKind::HighEntropy);
        }
        None
    }
}

fn split_trailing_ws(chunk: &str) -> (&str, &str) {
    let end = chunk.trim_end_matches(char::is_whitespace).len();
    (&chunk[..end], &chunk[end..])
}

fn is_jwt(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            part.len() >= 8
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        && token.starts_with("eyJ")
}

fn is_high_entropy(token: &str) -> bool {
    if token.len() < 32 {
        return false;
    }
    let alnum = token.chars().all(|c| c.is_ascii_alphanumeric());
    let has_upper = token.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = token.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    alnum && has_upper && has_lower && has_digit
}

fn sensitive_range(token: &str) -> Option<(usize, usize)> {
    let mut start = None;
    for (idx, ch) in token.char_indices() {
        let allowed = ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.';
        if allowed {
            start.get_or_insert(idx);
            continue;
        }
        if let Some(seg_start) = start.take() {
            let segment = &token[seg_start..idx];
            if Redactor::classify(segment).is_some() {
                return Some((seg_start, idx));
            }
        }
    }
    if let Some(seg_start) = start {
        let segment = &token[seg_start..];
        if Redactor::classify(segment).is_some() {
            return Some((seg_start, token.len()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLACEHOLDER_VALUE: &str = "[REDACTED]";

    #[test]
    fn redacts_known_token_formats() {
        let redactor = Redactor::new();
        let cases = [
            "AKIAIOSFODNN7EXAMPLE",
            "ghp_1234567890abcdefghijklmnopqrstuvwx",
            "sk-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRST",
            "sk-ant-api03-abcDEF123456_ghIJKL7890-mnopqrstuvwxYZ",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dozjgNryP4J3jVmNHl0w5N",
        ];
        for case in cases {
            let input = format!("prefix {case} suffix");
            let out = redactor.redact(&input);
            assert!(
                !out.contains(case),
                "token survived redaction: {case} -> {out}"
            );
            assert!(
                out.contains(PLACEHOLDER_VALUE),
                "no placeholder for {case}: {out}"
            );
        }
    }

    #[test]
    fn redacts_ssh_private_key_header() {
        let redactor = Redactor::new();
        let input = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk\n-----END OPENSSH PRIVATE KEY-----";
        let out = redactor.redact(input);
        assert!(!out.contains("b3BlbnNzaC1rZXk"), "key body survived: {out}");
    }

    #[test]
    fn redacts_high_entropy_strings() {
        let redactor = Redactor::new();
        let secret = "Zx9Kq2Lm8Vn4Pw7Rt6Yb3Hd1Gf5Js0Ca";
        let out = redactor.redact(&format!("token={secret}"));
        assert!(!out.contains(secret), "high-entropy string survived: {out}");
    }

    #[test]
    fn redaction_is_idempotent() {
        let redactor = Redactor::new();
        let input = "ghp_1234567890abcdefghijklmnopqrstuvwx and AKIAIOSFODNN7EXAMPLE";
        let once = redactor.redact(input);
        let twice = redactor.redact(&once);
        assert_eq!(once, twice, "redaction must be idempotent");
    }

    #[test]
    fn leaves_benign_text_unchanged() {
        let redactor = Redactor::new();
        let input = "the quick brown fox jumps over 12 lazy dogs";
        assert_eq!(redactor.redact(input), input);
    }

    #[test]
    fn preview_shows_redacted_text_and_match_count_before_send() {
        let redactor = Redactor::new();
        let bundle =
            "log line\ntoken ghp_1234567890abcdefghijklmnopqrstuvwx done\nAKIAIOSFODNN7EXAMPLE";
        let preview = redactor.preview(bundle);
        assert!(
            !preview
                .redacted
                .contains("ghp_1234567890abcdefghijklmnopqrstuvwx")
        );
        assert!(!preview.redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert_eq!(preview.match_count, 2, "two tokens should be flagged");
        assert_eq!(preview.original, bundle);
    }
}
