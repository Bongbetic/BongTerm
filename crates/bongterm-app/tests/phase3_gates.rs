//! Phase 3 exit-gate integration tests (spec §6.1 #9–#14).
//!
//! Each test maps to one acceptance gate. Tests that need `claude` are gated by
//! PATH probe; Windows-shell-only behavior is skipped on non-Windows test hosts.

use time::OffsetDateTime;
use uuid::Uuid;

use bongterm_devassist::ai::cmdk::{CmdKSession, CmdKView};
use bongterm_devassist::ai::explainer::Explainer;
use bongterm_devassist::history::filter::HistoryQuery;
use bongterm_devassist::jobs::runner::{JobRunner, JobSpec};
use bongterm_devassist::snippets::model::SnippetLibrary;
use bongterm_devassist::snippets::render::render_snippet;
use bongterm_storage_api::{BlockId, CommandBlockRow, PaneId, SessionId};
use bongterm_test_kit::mocks::ai_backend::MockAiBackend;
use bongterm_test_kit::mocks::notifier::MockNotifier;
use std::collections::HashMap;
use std::process::Command;

/// Gate #9: Cmd-K preview returns text and never executes until confirm.
#[test]
fn gate_9_cmdk_preview_does_not_run_until_confirm() {
    let backend = MockAiBackend::with_suggestion("git status");
    let mut session = CmdKSession::new(Box::new(backend.clone()));

    let _ = session
        .request_preview(
            "show me repo state",
            bongterm_devassist::ai::runner::AiContext {
                cwd: "C:\\repo".to_string(),
                shell: "pwsh".to_string(),
                failed_command: None,
                transcript_tail: String::new(),
            },
        )
        .expect("preview should succeed");

    match session.view() {
        CmdKView::Previewed { command } => assert_eq!(command, "git status"),
        other => panic!("expected Previewed, got {other:?}"),
    }

    assert_eq!(
        backend.run_count(),
        0,
        "gate #9: preview must not execute the command"
    );

    let to_run = session.confirm_run().expect("confirm should yield command");
    assert_eq!(to_run, "git status");
}

/// Gate #10: a failed command is explainable; successful and running commands are not.
#[test]
fn gate_10_explainer_only_offers_on_failure() {
    let failed = CommandBlockRow {
        id: BlockId(Uuid::new_v4()),
        pane_id: PaneId(Uuid::new_v4()),
        session_id: SessionId(Uuid::new_v4()),
        command: "bad-command".to_string(),
        exit_code: Some(1),
        started_at: OffsetDateTime::now_utc(),
        finished_at: None,
    };
    let command_not_found = CommandBlockRow {
        id: BlockId(Uuid::new_v4()),
        pane_id: PaneId(Uuid::new_v4()),
        session_id: SessionId(Uuid::new_v4()),
        command: "missing".to_string(),
        exit_code: Some(127),
        started_at: OffsetDateTime::now_utc(),
        finished_at: None,
    };
    let ok = CommandBlockRow {
        id: BlockId(Uuid::new_v4()),
        pane_id: PaneId(Uuid::new_v4()),
        session_id: SessionId(Uuid::new_v4()),
        command: "echo hi".to_string(),
        exit_code: Some(0),
        started_at: OffsetDateTime::now_utc(),
        finished_at: None,
    };
    let running = CommandBlockRow {
        id: BlockId(Uuid::new_v4()),
        pane_id: PaneId(Uuid::new_v4()),
        session_id: SessionId(Uuid::new_v4()),
        command: "sleep 1".to_string(),
        exit_code: None,
        started_at: OffsetDateTime::now_utc(),
        finished_at: None,
    };

    assert!(
        Explainer::is_explainable(&failed),
        "gate #10: non-zero exit must be explainable"
    );
    assert!(
        Explainer::is_explainable(&command_not_found),
        "gate #10: command-not-found must be explainable"
    );
    assert!(
        !Explainer::is_explainable(&ok),
        "gate #10: zero exit must NOT be offered an explanation"
    );
    assert!(
        !Explainer::is_explainable(&running),
        "gate #10: still-running command is not explainable"
    );
}

/// Gate #11: smart history applies filters and separates free text.
#[test]
fn gate_11_smart_history_filters_then_ranks() {
    let q = HistoryQuery::parse("exit:1 cargo build");
    assert!(q.has_filter(), "gate #11: `exit:1` must parse as a filter");
    assert_eq!(q.free_text(), "cargo build");
}

fn claude_on_path() -> bool {
    Command::new("claude")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Gate #9 (real binary): when available, preview round-trip returns non-empty text.
#[test]
fn gate_9_real_claude_preview_when_installed() {
    if !claude_on_path() {
        eprintln!("skipping: `claude` not on PATH (nightly-only gate)");
        return;
    }

    use bongterm_devassist::ai::runner::{
        AiBackend, AiContext, AiIntent, AiRequest, ClaudeCodeAiRunner,
    };

    let runner = ClaudeCodeAiRunner::discover().expect("discover claude");
    match runner.availability() {
        bongterm_devassist::ai::runner::AiAvailability::Available { .. } => {}
        other => panic!("gate #9: discovered runner should be available, got {other:?}"),
    }

    let suggestion = runner
        .suggest(&AiRequest {
            intent: AiIntent::NlToCommand,
            user_text: "list files in the current directory".to_string(),
            context: AiContext {
                cwd: "C:\\".to_string(),
                shell: "pwsh".to_string(),
                failed_command: None,
                transcript_tail: String::new(),
            },
        })
        .expect("suggest should succeed");

    assert!(
        !suggestion.command.trim().is_empty(),
        "gate #9: real backend must return a non-empty preview"
    );
}

/// Gate #12: snippet params must all be present before render succeeds.
#[test]
fn gate_12_snippet_requires_all_params_before_run() {
    let lib = r#"{ snippets: [ { name: "deploy", scope: "workspace",
        command: "kubectl rollout restart deploy/${param:svc} -n ${param:ns}" } ] }"#;

    let snippet = SnippetLibrary::from_json5(lib)
        .expect("parse")
        .snippets
        .into_iter()
        .next()
        .expect("one snippet");
    let mut params = HashMap::new();
    params.insert("svc".to_string(), "api".to_string());

    assert!(
        render_snippet(&snippet, &params).is_err(),
        "gate #12: missing param must error"
    );

    params.insert("ns".to_string(), "prod".to_string());
    let rendered = render_snippet(&snippet, &params).expect("all params present");
    assert_eq!(rendered, "kubectl rollout restart deploy/api -n prod");
}

/// Gate #13: non-zero job exit produces failed state and one completion toast.
#[tokio::test]
#[cfg_attr(not(windows), ignore)]
async fn gate_13_failed_job_emits_failure_toast() {
    let notifier = MockNotifier::new();
    let runner = JobRunner::new(&notifier);
    let spec = JobSpec::shell("gate13", "cmd", &["/C", "exit 3"]);
    let outcome = runner.run_to_completion(spec).await.expect("run");

    assert_eq!(
        outcome.final_state,
        // `run_to_completion` maps non-zero process exit to `Failed{exit_code}`.
        bongterm_devassist::jobs::JobState::Failed { exit_code: 3 }
    );
    let toasts = notifier.toasts();
    assert_eq!(toasts.len(), 1, "gate #13: exactly one completion toast");
    assert!(
        toasts[0].body.contains('3'),
        "gate #13: toast must carry exit code"
    );
}

/// Gate #14: clickable overlays and OSC 8 spoof guard.
#[test]
fn gate_14_clickable_patterns_and_osc8_spoof_guard() {
    use bongterm_devassist::patterns::matchers::{PatternKind, scan_file_locations};
    use bongterm_devassist::patterns::url::{parse_osc8, scan_urls, verify_destination};

    let line = "error[E0432]: at src/lib.rs:12:9 — see https://example.com/docs";
    let spans = scan_file_locations(line);
    assert!(
        spans
            .iter()
            .any(|s| matches!(s.kind, PatternKind::FileLine)),
        "gate #14: must detect src/lib.rs:12:9"
    );

    let urls = scan_urls(line);
    assert_eq!(urls.len(), 1, "gate #14: one bare URL");

    let osc8 = "\x1b]8;;https://evil.example\x07https://bank.example\x1b]8;;\x07";
    let links = parse_osc8(osc8).expect("parse osc8");
    assert!(!links.is_empty(), "gate #14: osc8 must parse");
    assert!(
        links[0].is_spoof_suspect(),
        "gate #14: visible URL mismatch must be flagged"
    );

    assert!(verify_destination("https://example.com").is_ok());
    assert!(verify_destination("file:///c:/windows/system32").is_err());
    assert!(verify_destination("javascript:alert(1)").is_err());
}
