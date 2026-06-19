# BongTerm Public Flip Checklist

status: blocked

## Required Before Public Visibility

| Gate | Status | Evidence |
| --- | --- | --- |
| README public-ready | pass | `README.md` |
| CONTRIBUTING present | pass | `CONTRIBUTING.md` |
| LICENSE present | pass | `LICENSE`, `LICENSE-APACHE` |
| CODE_OF_CONDUCT present | pass | `CODE_OF_CONDUCT.md` |
| Issue templates present | pass | `.github/ISSUE_TEMPLATE/` |
| PR template present | pass | `.github/PULL_REQUEST_TEMPLATE.md` |
| PRIVACY present | pass | `PRIVACY.md` |
| SECURITY inbox real and monitored | pass | GitHub private vulnerability reporting URL in `SECURITY.md`; owner/cadence recorded. |
| No unresolved placeholders in active release docs | pass | `SECURITY.md`, `README.md`, `INSTALL.md`, and release runbook use concrete public handles. Historical planning docs may still quote old placeholder examples. |
| No committed secrets in repo/history | pending | Run full-history scan before flip. |
| All P0 gates green for 7 consecutive nightlies | pass | Scheduled `nightly.yml` runs `27411817353`, `27463710495`, `27496013141`, `27549311099`, `27616935145`, `27687120185`, and `27755555379` passed on `master`. |
| Phase 5 clean-VM signed install smoke complete | blocked | External VM/signing proof required. |
| ADR-0002 product-name decision accepted | pending | Phase 6.C. |
| ADR-0009 trademark search accepted | pending | Phase 6.C. |
| Ship-when checklist PASS | blocked | `docs/runbook/release.md` |

## Flip Log

Do not flip repository public until every gate above is `pass`.
