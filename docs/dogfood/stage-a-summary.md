# BongTerm Stage A Summary

status: not_started
window_start:
window_end:
working_days_logged: 0
open_p0_p1_defects: unknown
confirmed_secret_leaks: unknown
exit_verdict: pending

## Preconditions

| Gate | Status | Evidence |
| --- | --- | --- |
| Phase 5 clean-VM signed install smoke accepted or complete | blocked | External VM/signing proof still required. |
| Remote nightly proof accepted or complete | blocked | Required remote-nightly proof still required. |
| Signed/installable build available | pending | Fill at Stage A start. |

## Workload Coverage Matrix

| Minimum | Week 1 | Week 2 | Week 3 | Week 4 | Week 5 | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Long-running command, >=1 per week | pending | pending | pending | pending | pending | Daily log links. |
| Failed-command explainer use, >=1 per week | pending | pending | pending | pending | pending | Daily log links. |
| Cmd-K use, >=1 per week | pending | pending | pending | pending | pending | Daily log links. |
| Shell switch, >=1 per week | pending | pending | pending | pending | pending | Daily log links. |
| Agent run, >=1 per working day | pending | pending | pending | pending | pending | Daily log links. |
| MCP server session, >=1 per week if MCP shipped | pending | pending | pending | pending | pending | Mark `N/A - MCP not in MVP-0` only if true. |
| Simulated crash/recovery drill, >=1 per week | pending | pending | pending | pending | pending | Daily log links. |

## Daily Logs

No Stage A daily logs started.

## Defect Reconciliation

Pending Stage A start.

## Secret-Leak Audit

Pending `docs/dogfood/secret-leak-audit.md` after dogfood corpus exists.

## Exit Verdict

Stage A is not started. Do not mark `PASS` until 30 working-day logs exist, all workload minimums are satisfied, open P0/P1 defects are zero, and secret-leak audit concludes zero confirmed leaks.
