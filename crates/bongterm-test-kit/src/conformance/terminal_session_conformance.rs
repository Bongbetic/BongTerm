//! Conformance suite for [`bongterm_term::TerminalSession`].

use bongterm_term::{TerminalProfile, TerminalSession};

/// Run happy-path conformance checks against any [`TerminalSession`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(session: &impl TerminalSession) {
    // Start with default profile must succeed.
    assert!(
        session.start(TerminalProfile::default()).is_ok(),
        "start(TerminalProfile::default()) must return Ok"
    );

    // Writing bytes to input must succeed after start.
    assert!(
        session.write_input(b"hello").is_ok(),
        "write_input(b\"hello\") must return Ok after start"
    );
}
