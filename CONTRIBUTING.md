# Contributing to BongTerm

BongTerm is pre-release. Contributions should preserve the MVP-0 scope and security boundaries.

## Local Checks

Run targeted checks first, then broader gates when touching shared surfaces:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --workspace -- -D warnings
cargo test --workspace
cargo xtask check-deps
```

Release/security surfaces may also require:

```powershell
cargo xtask secret-leak-corpus
cargo xtask prompt-injection-corpus
cargo xtask forbidden-abstraction
```

## Agent Profile PRs

Third-party agent profiles are community/import-only unless adopted as first-party. Profiles that launch external tools must include fixtures, adapter contract tests, and a security review note. Stale profiles may be marked unsupported.

## Scope

Do not pull post-MVP features into MVP-0: Markdown review, Command Lens, database branching, durable session daemon, plugin marketplace, cross-platform ports, or MCP host pooling.
