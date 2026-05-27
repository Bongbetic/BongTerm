//! Conformance suite for [`bongterm_agents::AgentAdapter`].

use bongterm_agents::AgentAdapter;

/// Run happy-path conformance checks against any [`AgentAdapter`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(adapter: &impl AgentAdapter) {
    let result = adapter.discover();
    assert!(result.found, "discover() must return found == true for a present adapter");

    let caps = adapter.capabilities();
    assert!(!caps.name.is_empty(), "capabilities().name must be non-empty");
}
