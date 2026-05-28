# BongTerm Handoff — 2026-05-29

## Resume instructions

1. Open `C:\Users\souba\Documents\Projects\BongT` in Claude Code.
2. Session start protocol (from `CLAUDE.md`): activate caveman ultra, read `graphify-out/graph_report.md` (absent — skip), read `orca.md`.
3. **User directive this session:** "Create all plans for Phases 2–6 first, then continue execution." Execute that before resuming Phase 1 tasks.

---

## What was done this session

| Commit | Task | Summary |
|--------|------|---------|
| `8508c11` | `mux/1.D.1` | `bongterm-mux` crate: `MuxRouter` port trait, `InMemoryMux`, `MockMuxRouter` (call spy). 29 tests. |
| `4c53109` | `mux/1.D.2` | Split h/v (`split_pane`), focus cycle (`focus_next_pane`), `Rect` type (replaces bare `cols/rows`). 47 tests. |
| `d03199f` | chore | `orca.md` Phase 1 status updated. |

All commits on `master`. Zero clippy warnings, fmt clean on all.

---

## Current state

- **`[next]` in `orca.md`:** `1.D.3 Layout save/restore`
- **Phase 1 status:** 🔨 In progress — `1.C.1–5` + `1.D.1–2` done.
- **Blocked task:** `1.B.3` (`WezTermAdapter::ingest_bytes`) — blocked on wezterm submodule gitlink. Fix command in `docs/adr/0007-wezterm-submodule.md` § "Fix required before Phase 1.B.3".
- **Pending in orca.md (non-blocked, pre-`[next]`):** `1.A.4b` — SettingsWriter port + `FileSettingsProvider::write`.

---

## User directive: create all plans before execution

User asked: *"create all the plans first and then proceed with execution."*

Phases needing TDD-level plans:

| Phase | Spec gates | Outline in `orca.md` |
|-------|-----------|----------------------|
| Phase 2 — Agent Observability | §6.1 #15, #24 | Lines ~143–160 |
| Phase 3 — Developer UX | §6.1 #9–14 | Lines ~162–185 |
| Phase 4 — MCP + Secrets + Security | §6.1 #16, #19, #23, #31 | Lines ~187–214 |
| Phase 5 — Hardening + Release | §6.1 #18, #20, #21, #25, #26, #30 | Lines ~216–246 |
| Phase 6 — Dogfood → Public | §6.1 #22 + §6.6 | Lines ~248–269 |

Existing plan files:
- Phase 0: `docs/superpowers/plans/2026-05-27-bongt-mvp0.md`
- Phase 1: `docs/superpowers/plans/2026-05-28-bongt-phase1.md`

**Recommended approach (offered to user, not yet chosen):**
- **Parallel workflow** — 5 agents fan out, one per phase, each reads PRD + writes plan. Fastest. Requires user to include "workflow" in prompt.
- **Sequential** — one plan per session. Slower but each plan gets full context.

Note: `orca.md` says to also query the AnythingLLM `engineer` workspace for each phase re-plan. That's an external service — Claude cannot access it. User should supplement plans with insights from that workspace after generation.

---

## Key artifacts

| Artifact | Path |
|----------|------|
| Authoritative spec (1063 lines, §0–§23) | `docs/PRD/bongterm_prd_v7.md` |
| Canonical design doc | `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` |
| Task authority | `orca.md` |
| Phase 1 plan | `docs/superpowers/plans/2026-05-28-bongt-phase1.md` |
| ADRs (0003–0007, all Accepted) | `docs/adr/` |
| `bongterm-mux` implementation | `crates/bongterm-mux/src/lib.rs` |
| Session memory | `C:\Users\souba\.claude\projects\C--Users-souba-Documents-Projects-BongT\memory\` |

---

## Architecture constraints to keep in mind

- No `wezterm_mux` consumption — BongTerm owns pane lifecycle (ADR-007).
- `bongterm-mux` is purely structural: no PTY sessions, no rendering, no input routing.
- Hot-path rules: no sync I/O, no allocs, no agent/MCP calls — see `CLAUDE.md` §Terminal hot-path rules.
- Module ownership matrix is binding — see `CLAUDE.md` §Architectural contract.

---

## Suggested skills for next session

```
superpowers:writing-plans      — invoke for each Phase 2–6 plan (per user directive)
superpowers:test-driven-development  — invoke before implementing any task
superpowers:verification-before-completion  — invoke before marking any task done
superpowers:subagent-driven-development     — if parallelising independent tasks
caveman:caveman                — ultra mode, active from session start per CLAUDE.md
```

If user opts into workflow for parallel plan creation:
```
# In the prompt, include the word "workflow" to trigger Workflow tool opt-in
# Then fan out 5 agents: one per phase, each reads PRD §6.1 acceptance criteria
# and the phase outline from orca.md, produces TDD-level plan file
```

---

## Immediate next actions (in order)

1. **If creating all plans first (user directive):**
   - Ask user: sequential or workflow (parallel)?
   - For each phase: read PRD §6.1 gates + orca.md outline → invoke `superpowers:writing-plans` → write to `docs/superpowers/plans/YYYY-MM-DD-bongt-phase{N}.md`
   - Update orca.md phase outline with plan file reference once done

2. **Then continue Phase 1 execution:**
   - `[next]` = `1.D.3 Layout save/restore` — extends `bongterm-mux` with serializable layout snapshots (JSON5 or similar), save/restore via the `bongterm-settings` crate's write port
   - `1.A.4b` — SettingsWriter port + `FileSettingsProvider::write` (no `[next]` marker but unremoved; clarify with user if still needed)
   - Then 1.E (shell integration), 1.F (resource dashboard), 1.G (SQLite)

---

*Generated by Claude Sonnet 4.6 on 2026-05-29. Relocated to workspace root 2026-05-29. Sensitive fields: none.*
