//! Gate #15 integration test: Claude Code + Codex CLI launch contract,
//! sidebar status projection, and transcript capture — offline (no binary
//! required). See spec §6.1 #15.
//!
//! Composes units delivered by Tasks 2.A.4, 2.A.5, 2.B.1, 2.C.2a against their
//! real APIs: the launch contract (`build_process_spec`), the streamed output
//! surfacing as `AgentEvent::Output` (the raw material a `TranscriptSink`
//! persists — sink persistence is unit-tested in `transcript.rs`), and the
//! lifecycle reaching a terminal state.

use bongterm_agents::claude_code::ClaudeCodeAdapter;
use bongterm_agents::codex_cli::CodexCliAdapter;
use bongterm_agents::lifecycle::{AgentLifecycle, LifecycleCommand, LifecycleState};
use bongterm_agents::{AgentAdapter, AgentEvent, ExitState, OutputChunk};

/// Drive an adapter's real classifier with fixture bytes, capturing the streamed
/// output into a transcript and returning (transcript_text, saw_completed).
fn run_offline(adapter: &dyn AgentAdapter, fixture: &[u8]) -> (String, bool) {
    // (a) Launch contract: building a process spec must succeed with a binary.
    let spec = adapter
        .build_process_spec("C:/work/repo", "summarize the failing test")
        .expect("adapter must build a process spec");
    assert!(
        !spec.launch.binary.is_empty(),
        "launch binary must be set for {}",
        adapter.capabilities().name
    );

    // (b)+(c): pump fixture bytes through the real classifier; non-structured
    // lines surface as Output events — exactly what the transcript captures.
    let mut classifier = adapter.create_classifier();
    let mut rx = classifier.event_receiver();
    classifier.ingest(&OutputChunk {
        bytes: fixture.to_vec(),
        from_stderr: false,
    });
    let _summary = classifier.finalize(ExitState::Clean { exit_code: 0 });

    let mut transcript = String::new();
    let mut saw_completed = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            AgentEvent::Output(chunk) => {
                transcript.push_str(&String::from_utf8_lossy(&chunk.bytes));
            }
            AgentEvent::Completed { .. } => saw_completed = true,
            _ => {}
        }
    }
    (transcript, saw_completed)
}

#[test]
fn claude_code_launch_and_transcript_capture() {
    let adapter = ClaudeCodeAdapter::new();
    assert_eq!(adapter.capabilities().name, "claude-code");

    let fixture = br#"{"type":"text","text":"hello from claude"}
{"type":"result","exit_code":0}
"#;
    let (transcript, _saw_completed) = run_offline(&adapter, fixture);
    assert!(
        transcript.contains("hello from claude"),
        "transcript must capture streamed output, got: {transcript}"
    );
}

#[test]
fn codex_cli_launch_and_transcript_capture() {
    let adapter = CodexCliAdapter::new();
    assert_eq!(adapter.capabilities().name, "codex-cli");

    let fixture = b"[tool] shell: ls -la\nhello from codex\n";
    let (transcript, _saw_completed) = run_offline(&adapter, fixture);
    assert!(
        transcript.contains("hello from codex"),
        "transcript must capture streamed output, got: {transcript}"
    );
}

/// Sidebar status projection: a fresh run is Running; after a process exit the
/// lifecycle must reach the terminal Exited state (never stuck in Running).
#[test]
fn sidebar_status_reaches_terminal_state() {
    let mut lc = AgentLifecycle::new();
    lc.apply(LifecycleCommand::Launch).expect("launch");
    assert_eq!(lc.state(), LifecycleState::Running);
    lc.apply(LifecycleCommand::ProcessExited)
        .expect("observe process exit");
    assert_eq!(lc.state(), LifecycleState::Exited);
}

/// Truly-installed smoke test. Skipped unless BONGTERM_E2E_AGENTS=1 and the
/// binary is on PATH. Never gates CI on agent presence.
#[test]
#[ignore = "requires an installed agent binary; opt in via BONGTERM_E2E_AGENTS=1"]
fn claude_code_real_binary_discovers() {
    if std::env::var("BONGTERM_E2E_AGENTS").as_deref() != Ok("1") {
        return;
    }
    let adapter = ClaudeCodeAdapter::new();
    let result = adapter.discover();
    assert!(result.found, "claude binary expected on PATH for E2E run");
}
