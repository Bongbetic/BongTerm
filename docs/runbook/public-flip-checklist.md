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
| SECURITY inbox real and monitored | blocked | `SECURITY.md` still needs real address/owner. |
| No PLACEHOLDER tokens repo-wide | blocked | Real security inbox required first. |
| No committed secrets in repo/history | pending | Run full-history scan before flip. |
| All P0 gates green for 7 consecutive nightlies | blocked | Remote proof not complete. |
| Phase 5 clean-VM signed install smoke complete | blocked | External VM/signing proof required. |
| ADR-0002 product-name decision accepted | pending | Phase 6.C. |
| ADR-0009 trademark search accepted | pending | Phase 6.C. |
| Ship-when checklist PASS | blocked | `docs/runbook/release.md` |

## Flip Log

Do not flip repository public until every gate above is `pass`.
