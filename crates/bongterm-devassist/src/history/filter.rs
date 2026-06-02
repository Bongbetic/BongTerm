//! Smart-history filter parsing (gate #11).
//!
//! Supported filters: `cwd:` `branch:` `agent:` `exit:` `time:` `shell:`
//! `duration:`. Parsing runs off the terminal hot path.

/// Metadata about a history entry, used for matching.
#[derive(Debug, Clone)]
pub struct HistoryEntryMeta {
    pub command: String,
    pub cwd: String,
    pub branch: Option<String>,
    pub agent: Option<String>,
    pub exit_code: Option<i64>,
    pub shell: String,
    pub duration_secs: f64,
    /// How long ago the command ran, in seconds.
    pub age_secs: u64,
}

/// The closed set of supported smart-history filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterKind {
    Cwd,
    Branch,
    Agent,
    Exit,
    Time,
    Shell,
    Duration,
}

impl FilterKind {
    /// The textual prefix, without the trailing colon.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            FilterKind::Cwd => "cwd",
            FilterKind::Branch => "branch",
            FilterKind::Agent => "agent",
            FilterKind::Exit => "exit",
            FilterKind::Time => "time",
            FilterKind::Shell => "shell",
            FilterKind::Duration => "duration",
        }
    }

    const ALL: [FilterKind; 7] = [
        FilterKind::Cwd,
        FilterKind::Branch,
        FilterKind::Agent,
        FilterKind::Exit,
        FilterKind::Time,
        FilterKind::Shell,
        FilterKind::Duration,
    ];
}

/// A parsed smart-history query: extracted filters plus remaining free text.
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    filters: Vec<(FilterKind, String)>,
    pub free_text: String,
}

impl HistoryQuery {
    /// Parse a raw query string.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let mut filters = Vec::new();
        let mut free = Vec::new();

        for token in input.split_whitespace() {
            if let Some((prefix, value)) = token.split_once(':')
                && let Some(kind) = FilterKind::ALL
                    .iter()
                    .copied()
                    .find(|kind| kind.prefix() == prefix)
                && !value.is_empty()
            {
                filters.push((kind, value.to_string()));
                continue;
            }

            free.push(token);
        }

        Self {
            filters,
            free_text: free.join(" "),
        }
    }

    /// The value for a given filter, if present.
    #[must_use]
    pub fn filter(&self, kind: FilterKind) -> Option<&str> {
        self.filters
            .iter()
            .find(|(filter_kind, _)| *filter_kind == kind)
            .map(|(_, value)| value.as_str())
    }

    /// Whether any filter token was parsed.
    #[must_use]
    pub fn has_filter(&self) -> bool {
        !self.filters.is_empty()
    }

    /// The free text to match against command text.
    #[must_use]
    pub fn free_text(&self) -> &str {
        &self.free_text
    }

    /// Whether an entry satisfies all filters and free text.
    #[must_use]
    pub fn matches(&self, entry: &HistoryEntryMeta) -> bool {
        if !self.free_text.is_empty() && !entry.command.contains(&self.free_text) {
            return false;
        }

        self.filters.iter().all(|(kind, value)| match kind {
            FilterKind::Cwd => entry.cwd.contains(value.as_str()),
            FilterKind::Branch => entry.branch.as_deref() == Some(value.as_str()),
            FilterKind::Agent => entry.agent.as_deref() == Some(value.as_str()),
            FilterKind::Exit => entry
                .exit_code
                .is_some_and(|exit_code| exit_code.to_string() == *value),
            FilterKind::Time => {
                parse_window_secs(value).is_some_and(|window| entry.age_secs <= window)
            }
            FilterKind::Shell => entry.shell == *value,
            FilterKind::Duration => match_duration(value, entry.duration_secs),
        })
    }
}

/// Parse a window like `24h`, `30m`, or `45s` into seconds.
fn parse_window_secs(s: &str) -> Option<u64> {
    let (num, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: u64 = num.parse().ok()?;
    match unit {
        "s" => Some(n),
        "m" => Some(n * 60),
        "h" => Some(n * 3600),
        "d" => Some(n * 86400),
        _ => None,
    }
}

/// Match a duration spec like `>5s`, `<5m`, or `>=10s`.
fn match_duration(spec: &str, value_secs: f64) -> bool {
    let (op, rest) = if let Some(rest) = spec.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = spec.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = spec.strip_prefix('>') {
        (">", rest)
    } else if let Some(rest) = spec.strip_prefix('<') {
        ("<", rest)
    } else {
        ("==", spec)
    };

    let Some(threshold) = parse_window_secs(rest) else {
        return false;
    };
    let threshold = threshold as f64;

    match op {
        ">" => value_secs > threshold,
        "<" => value_secs < threshold,
        ">=" => value_secs >= threshold,
        "<=" => value_secs <= threshold,
        _ => (value_secs - threshold).abs() < f64::EPSILON,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_filter_kind() {
        let q = HistoryQuery::parse(
            "cwd:C:\\proj branch:main agent:claude exit:1 time:24h shell:pwsh duration:>5s build",
        );
        assert_eq!(q.free_text, "build");
        assert_eq!(q.filter(FilterKind::Cwd), Some("C:\\proj"));
        assert_eq!(q.filter(FilterKind::Branch), Some("main"));
        assert_eq!(q.filter(FilterKind::Agent), Some("claude"));
        assert_eq!(q.filter(FilterKind::Exit), Some("1"));
        assert_eq!(q.filter(FilterKind::Time), Some("24h"));
        assert_eq!(q.filter(FilterKind::Shell), Some("pwsh"));
        assert_eq!(q.filter(FilterKind::Duration), Some(">5s"));
    }

    #[test]
    fn unknown_prefix_stays_free_text() {
        let q = HistoryQuery::parse("foo:bar cargo");
        assert_eq!(q.free_text, "foo:bar cargo");
        assert_eq!(q.filter(FilterKind::Cwd), None);
    }

    #[test]
    fn matches_applies_all_filters_conjunctively() {
        let q = HistoryQuery::parse("shell:pwsh exit:0 build");
        let entry = HistoryEntryMeta {
            command: "cargo build".to_string(),
            cwd: "C:\\proj".to_string(),
            branch: Some("main".to_string()),
            agent: None,
            exit_code: Some(0),
            shell: "pwsh".to_string(),
            duration_secs: 12.0,
            age_secs: 60,
        };
        assert!(q.matches(&entry));

        let q2 = HistoryQuery::parse("shell:cmd build");
        assert!(!q2.matches(&entry));
    }

    #[test]
    fn duration_and_time_comparators() {
        let entry = HistoryEntryMeta {
            command: "sleep".to_string(),
            cwd: String::new(),
            branch: None,
            agent: None,
            exit_code: Some(0),
            shell: "bash".to_string(),
            duration_secs: 10.0,
            age_secs: 3600,
        };
        assert!(HistoryQuery::parse("duration:>5s").matches(&entry));
        assert!(!HistoryQuery::parse("duration:>5m").matches(&entry));
        assert!(HistoryQuery::parse("time:24h").matches(&entry));
        assert!(!HistoryQuery::parse("time:30m").matches(&entry));
    }
}
