//! Conformance suites for `BongTerm` port traits.
//!
//! Each submodule exposes one or more `run_*_conformance` functions that
//! exercise happy-path invariants against any implementation of the
//! corresponding port trait. Pass a mock or real implementation; the suite
//! asserts the contract holds.

pub mod agent_adapter_conformance;
pub mod frecency_repo_conformance;
pub mod mcp_transport_conformance;
pub mod policy_evaluator_conformance;
pub mod process_governor_conformance;
pub mod renderer_backend_conformance;
pub mod secret_store_conformance;
pub mod settings_provider_conformance;
pub mod storage_repository_conformance;
pub mod terminal_session_conformance;

pub mod negative;
