//! Conformance suite for [`bongterm_process_control::ProcessGovernor`].

use bongterm_process_control::{JobObjectCaps, ProcessGovernor, ProcessHandle};

/// Run happy-path conformance checks against any [`ProcessGovernor`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(governor: &impl ProcessGovernor) {
    let handle = ProcessHandle(99_999);

    assert!(
        governor.attach(handle, JobObjectCaps::UNLIMITED).is_ok(),
        "attach() with UNLIMITED caps must return Ok"
    );

    assert!(
        governor.sample_rss(handle).is_ok(),
        "sample_rss() for an attached process must return Ok"
    );
}
