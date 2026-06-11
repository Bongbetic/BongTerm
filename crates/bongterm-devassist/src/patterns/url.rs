//! URL detection and OSC 8 hyperlink parsing with spoof guard.
//!
//! Links are overlay-only and never auto-opened. Destination verification rejects
//! non-http(s) schemes before the UI offers navigation.

use crate::DevassistError;
use regex::Regex;
use std::sync::LazyLock;

/// Kind of detected link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// Bare http(s) URL in plain text.
    Bare,
    /// OSC 8 hyperlink carrying URI and display text.
    Osc8,
}

/// A bare URL overlay span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlSpan {
    pub url: String,
    pub kind: LinkKind,
    pub start: usize,
    pub end: usize,
}

/// Parsed OSC 8 hyperlink data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Osc8Link {
    pub url: String,
    pub text: String,
    pub kind: LinkKind,
}

impl Osc8Link {
    /// Flag display text that looks like a URL for a different host.
    #[must_use]
    pub fn is_spoof_suspect(&self) -> bool {
        let text = self.text.trim();
        if text.starts_with("http://") || text.starts_with("https://") {
            host_of(text) != host_of(&self.url)
        } else {
            false
        }
    }
}

fn host_of(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()?
        .split('@')
        .next_back()?;
    Some(host.to_ascii_lowercase())
}

static RE_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://[^\s\x1b]+").expect("valid url regex"));

/// Scan a line for bare http(s) URLs.
#[must_use]
pub fn scan_urls(line: &str) -> Vec<UrlSpan> {
    RE_URL
        .find_iter(line)
        .map(|matched| UrlSpan {
            url: matched.as_str().to_string(),
            kind: LinkKind::Bare,
            start: matched.start(),
            end: matched.end(),
        })
        .collect()
}

/// Parse OSC 8 hyperlinks from raw text with escape sequences.
#[allow(clippy::unnecessary_wraps)]
pub fn parse_osc8(raw: &str) -> Result<Vec<Osc8Link>, DevassistError> {
    let mut links = Vec::new();
    let open = "\x1b]8;";
    let mut search_from = 0;

    while let Some(relative_start) = raw[search_from..].find(open) {
        let start = search_from + relative_start;
        let after_open = &raw[start + open.len()..];
        let Some(params_end) = after_open.find(';') else {
            break;
        };
        let after_uri = &after_open[params_end + 1..];
        let Some((st_start, st_end)) = find_st(after_uri) else {
            break;
        };

        let uri = after_uri[..st_start].to_string();
        let after_st = &after_uri[st_end..];
        let text_end = after_st.find(open).unwrap_or(after_st.len());
        let text = after_st[..text_end].to_string();
        if !uri.is_empty() {
            links.push(Osc8Link {
                url: uri,
                text,
                kind: LinkKind::Osc8,
            });
        }

        search_from = start + open.len() + params_end + 1 + st_end + text_end;
    }

    Ok(links)
}

fn find_st(text: &str) -> Option<(usize, usize)> {
    if let Some(position) = text.find("\x1b\\") {
        return Some((position, position + 2));
    }
    text.find('\x07').map(|position| (position, position + 1))
}

/// Verify a link destination is safe to offer for navigation.
pub fn verify_destination(url: &str) -> Result<(), DevassistError> {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        Ok(())
    } else {
        Err(DevassistError::Parse(format!(
            "refusing non-http(s) link destination: {url}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bare_urls() {
        let spans = scan_urls("see https://example.com/docs and http://localhost:3000 now");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].url, "https://example.com/docs");
        assert_eq!(spans[0].kind, LinkKind::Bare);
        assert_eq!(spans[1].url, "http://localhost:3000");
    }

    #[test]
    fn parses_osc8_hyperlink() {
        let raw = "\x1b]8;;https://example.com\x1b\\Example\x1b]8;;\x1b\\";
        let links = parse_osc8(raw).expect("parse osc8");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://example.com");
        assert_eq!(links[0].text, "Example");
        assert_eq!(links[0].kind, LinkKind::Osc8);
    }

    #[test]
    fn osc8_spoof_text_url_mismatch_is_flagged() {
        let link = Osc8Link {
            url: "https://evil.test/login".to_string(),
            text: "https://bank.example.com".to_string(),
            kind: LinkKind::Osc8,
        };
        assert!(link.is_spoof_suspect());
    }

    #[test]
    fn osc8_matching_text_is_not_flagged() {
        let link = Osc8Link {
            url: "https://example.com/x".to_string(),
            text: "Example docs".to_string(),
            kind: LinkKind::Osc8,
        };
        assert!(!link.is_spoof_suspect());
    }

    #[test]
    fn verify_destination_rejects_non_http_schemes() {
        assert!(verify_destination("https://example.com").is_ok());
        assert!(verify_destination("http://example.com").is_ok());
        assert!(verify_destination("file:///etc/passwd").is_err());
        assert!(verify_destination("javascript:alert(1)").is_err());
    }
}
