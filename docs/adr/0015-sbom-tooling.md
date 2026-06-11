# ADR-0015: SBOM Tooling

Status: Accepted

Decision: `xtask sbom` emits CycloneDX JSON from Cargo metadata and includes the vendored WezTerm component.

Reason: A repo-owned command gives stable CI output; a future `cargo-cyclonedx` switch can replace internals without changing the release gate.
