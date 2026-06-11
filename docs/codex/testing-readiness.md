# BongTerm Testing Readiness

Date: 2026-06-03
Status: ready for local/tester smoke after PR CI review; not ready for public release.

## Resolved For Testing

- Remote branch pushed over SSH: `codex/phase5-hardening-closeout`.
- PR created: https://github.com/soubarnak/BongTerm/pull/1
- SECURITY placeholder removed; vulnerability intake uses GitHub private reporting.
- Local Phase 6 prep docs and release verifier tooling are present.
- Windows SDK `makeappx.exe` found and used to produce `target/msix/BongTerm.msix`.
- Dev self-signed code-signing cert created in CurrentUser `My`, package signed, public cert exported to `target/msix/BongTerm-Dev.cer`.

## Testing Scope

Allowed:

- Local developer smoke tests.
- PR CI verification.
- Unsigned or dev-channel package smoke where the tester explicitly accepts that it is not a public release artifact.
- Dev-signed MSIX smoke with `target/msix/BongTerm-Dev.cer` imported into the tester trust store.
- Stage A practice logging that does not count toward public-release exit unless all Stage A preconditions are later satisfied.

Not allowed to claim:

- Phase 6 complete.
- Public release ready.
- Signed clean-VM install proof complete.
- Seven-nightly proof complete.

## Remaining Public-Release Gates

- Clean-VM signed install/upgrade/uninstall smoke.
- Seven consecutive remote nightly runs.
- Stage A 30 working days.
- Stage B 3-5 users for 14 days, or accepted experimental downgrade.
- Trademark/legal ADRs accepted.
- Signed `dist/` verified by `cargo xtask release-verify dist`.
- Public repository flip and GitHub release publication.
