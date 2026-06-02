//! File:line clickable-pattern matchers for Node/Python/Rust/.NET/TS.
//!
//! Produces overlay spans only; it does not mutate terminal scrollback.

use regex::Regex;
use std::sync::LazyLock;

/// Closed set of recognized file-location patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKind {
    /// `path:line[:col]` for compiler and Node-style frames.
    FileLine,
    /// Python traceback: `File "path", line N`.
    PythonTraceback,
    /// .NET stack frame: `in path:line N`.
    DotNetStack,
}

/// A matched file location as an overlay byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSpan {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub kind: PatternKind,
    pub start: usize,
    pub end: usize,
}

static RE_FILE_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?P<path>(?:[A-Za-z]:)?[\w./\\-]*[\w-]+\.(?:rs|ts|tsx|js|jsx|mjs|cjs|cs|go|py|java|kt|rb|c|h|cpp|hpp))(?::(?P<line>\d+))(?::(?P<col>\d+))?",
    )
    .expect("valid file:line regex")
});

static RE_PY_TRACEBACK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"File "(?P<path>[^"]+)", line (?P<line>\d+)"#).expect("valid py traceback regex")
});

static RE_DOTNET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"in (?P<path>(?:[A-Za-z]:)?[\w./\\-]+\.\w+):line (?P<line>\d+)")
        .expect("valid dotnet regex")
});

/// Scan one output line for clickable file locations.
#[must_use]
pub fn scan_file_locations(line: &str) -> Vec<FileSpan> {
    let mut spans = Vec::new();

    for capture in RE_DOTNET.captures_iter(line) {
        let matched = capture.get(0).expect("full match");
        spans.push(FileSpan {
            path: capture["path"].to_string(),
            line: capture
                .name("line")
                .and_then(|value| value.as_str().parse().ok()),
            column: None,
            kind: PatternKind::DotNetStack,
            start: matched.start(),
            end: matched.end(),
        });
    }

    for capture in RE_PY_TRACEBACK.captures_iter(line) {
        let matched = capture.get(0).expect("full match");
        spans.push(FileSpan {
            path: capture["path"].to_string(),
            line: capture
                .name("line")
                .and_then(|value| value.as_str().parse().ok()),
            column: None,
            kind: PatternKind::PythonTraceback,
            start: matched.start(),
            end: matched.end(),
        });
    }

    for capture in RE_FILE_LINE.captures_iter(line) {
        let matched = capture.get(0).expect("full match");
        let overlaps_existing = spans
            .iter()
            .any(|span: &FileSpan| matched.start() < span.end && span.start < matched.end());
        if overlaps_existing {
            continue;
        }

        spans.push(FileSpan {
            path: capture["path"].to_string(),
            line: capture
                .name("line")
                .and_then(|value| value.as_str().parse().ok()),
            column: capture
                .name("col")
                .and_then(|value| value.as_str().parse().ok()),
            kind: PatternKind::FileLine,
            start: matched.start(),
            end: matched.end(),
        });
    }

    spans.sort_by_key(|span| span.start);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_typescript_file_line_col() {
        let spans = scan_file_locations("error at src/index.ts:42:7 unexpected token");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].path, "src/index.ts");
        assert_eq!(spans[0].line, Some(42));
        assert_eq!(spans[0].column, Some(7));
        assert_eq!(spans[0].kind, PatternKind::FileLine);
        assert_eq!(
            &"error at src/index.ts:42:7 unexpected token"[spans[0].start..spans[0].end],
            "src/index.ts:42:7"
        );
    }

    #[test]
    fn matches_rust_file_line() {
        let spans = scan_file_locations("  --> crates/foo/src/lib.rs:128:13");
        assert_eq!(spans[0].path, "crates/foo/src/lib.rs");
        assert_eq!(spans[0].line, Some(128));
        assert_eq!(spans[0].column, Some(13));
    }

    #[test]
    fn matches_python_traceback() {
        let spans = scan_file_locations(r#"  File "app/main.py", line 10, in <module>"#);
        assert_eq!(spans[0].path, "app/main.py");
        assert_eq!(spans[0].line, Some(10));
        assert_eq!(spans[0].kind, PatternKind::PythonTraceback);
    }

    #[test]
    fn matches_node_stack_frame() {
        let spans = scan_file_locations("    at Object.<anonymous> (/srv/app/server.js:23:9)");
        assert_eq!(spans[0].path, "/srv/app/server.js");
        assert_eq!(spans[0].line, Some(23));
        assert_eq!(spans[0].column, Some(9));
    }

    #[test]
    fn matches_dotnet_stack_frame() {
        let spans = scan_file_locations(r"   at App.Main() in C:\proj\Program.cs:line 55");
        assert_eq!(spans[0].path, r"C:\proj\Program.cs");
        assert_eq!(spans[0].line, Some(55));
        assert_eq!(spans[0].kind, PatternKind::DotNetStack);
    }

    #[test]
    fn no_false_positive_on_plain_text() {
        let spans = scan_file_locations("the time is 12:30 and all is well");
        assert!(spans.is_empty(), "time-of-day must not match as file:line");
    }
}
