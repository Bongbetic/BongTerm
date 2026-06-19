# BongTerm Testing Readiness

Date: 2026-06-19
Status: ready for local unsigned-package smoke; not ready for public release.

## Resolved For Testing

- Remote branch pushed over SSH: `codex/phase5-hardening-closeout`.
- PR created: https://github.com/soubarnak/BongTerm/pull/1
- SECURITY placeholder removed; vulnerability intake uses GitHub private reporting.
- Local Phase 6 prep docs and release verifier tooling are present.
- Remote scheduled-nightly gate is complete: scheduled runs `27411817353`,
  `27463710495`, `27496013141`, `27549311099`, `27616935145`,
  `27687120185`, and `27755555379` passed on `master`.
- Windows SDK `makeappx.exe` found and used on 2026-06-19 to produce a real
  unsigned `target/msix/BongTerm.msix`. Set `BONGT_SIGN_THUMBPRINT` to produce
  a signed release package.

## Testing Scope

Allowed:

- Local developer smoke tests.
- PR CI verification.
- Unsigned or dev-channel package smoke where the tester explicitly accepts that it is not a public release artifact.
- Signed package smoke after `cargo xtask package-msix` runs with `BONGT_SIGN_THUMBPRINT` and the matching public certificate is trusted by the tester.
- Stage A practice logging that does not count toward public-release exit unless all Stage A preconditions are later satisfied.

Not allowed to claim:

- Phase 6 complete.
- Public release ready.
- Signed clean-VM install proof complete.

## Remaining Public-Release Gates

- Clean-VM signed install/upgrade/uninstall smoke.
- Stage A 30 working days.
- Stage B 3-5 users for 14 days, or accepted experimental downgrade.
- Trademark/legal ADRs accepted.
- Signed `dist/` verified by `cargo xtask release-verify dist`.
- Public repository flip and GitHub release publication.
