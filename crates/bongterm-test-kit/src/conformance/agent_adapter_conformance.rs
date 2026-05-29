//! Conformance suite for [`bongterm_agents::AgentAdapter`].

use bongterm_agents::{AgentAdapter, ExitState, OutputChunk};

/// Online conformance: requires the agent binary to be discoverable.
///
/// # Panics
/// Panics if any conformance assertion fails.
pub fn run(adapter: &impl AgentAdapter) {
    let result = adapter.discover();
    assert!(result.found, "discover() must return found == true");
    run_offline(adapter);
}

/// Offline conformance: exercises the parts of the contract that do not
/// depend on the binary being installed. Safe to run in CI without the CLI.
///
/// # Panics
/// Panics if any conformance assertion fails.
pub fn run_offline(adapter: &impl AgentAdapter) {
    let caps = adapter.capabilities();
    assert!(!caps.name.is_empty(), "capabilities().name must be non-empty");

    assert!(
        adapter.build_process_spec("C:\\x", "").is_err(),
        "empty prompt must be rejected"
    );
    let spec = adapter
        .build_process_spec("C:\\x", "do a thing")
        .expect("non-empty prompt must build a spec");
    assert!(!spec.launch.binary.is_empty(), "binary must be set");
    assert_eq!(spec.launch.cwd.as_deref(), Some("C:\\x"));

    let mut classifier = adapter.create_classifier();
    let _rx = classifier.event_receiver();
    classifier.ingest(&OutputChunk {
        bytes: b"hello\n".to_vec(),
        from_stderr: false,
    });
    let summary = classifier.finalize(ExitState::Clean { exit_code: 0 });
    assert!(
        summary.output_bytes >= 6,
        "finalize must report ingested byte count"
    );
    assert!(
        summary.replay_summary.is_some(),
        "finalize must populate replay_summary"
    );
}
