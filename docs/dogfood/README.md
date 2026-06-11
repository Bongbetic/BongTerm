# BongTerm Stage A Dogfood Protocol

Stage A validates the MVP-0 dogfood gate before any public release claim.

## Preconditions

- Phase 5 local gates are green.
- Signed/installable BongTerm build exists.
- External Phase 5 clean-VM signed install smoke is accepted or complete.
- Required remote nightly proof is accepted or complete.

Do not start Stage A daily logging until those preconditions are met. Prep files can exist before the clock starts.

## Stage A Rules

- Run BongTerm as the default terminal for 30 consecutive working days.
- Log every working day in `docs/dogfood/<YYYY-MM-DD>.md`.
- Any fallback to another terminal must be logged with tool, reason, duration, and whether it is a BongTerm blocker.
- Each daily log must include `bongterm_default`, `workloads_today`, `fallbacks`, `defects_filed`, and `secret_leak_check`.
- Any P0/P1 terminal-correctness defect blocks Stage A exit until fixed and revalidated.
- Any confirmed secret leak blocks Stage A exit until fixed, rescanned, and recorded as zero confirmed leaks.

## Workload Minimums

Track these in the daily logs and roll them up weekly in `stage-a-summary.md`.

1. At least one long-running command per week.
2. At least one failed-command explainer use per week.
3. At least one Cmd-K use per week.
4. At least one shell switch per week.
5. At least one agent run per working day.
6. At least one MCP server session per week if MCP shipped in MVP-0.
7. At least one simulated crash/recovery drill per week.

## Exit Evidence

Stage A exit requires:

- 30 working-day logs present, with any non-working gaps explained.
- All workload minimums satisfied.
- Zero open P0/P1 terminal-correctness defects.
- `docs/dogfood/secret-leak-audit.md` concludes zero confirmed secret leaks.
- `docs/dogfood/stage-a-summary.md` exit verdict is `PASS`.
