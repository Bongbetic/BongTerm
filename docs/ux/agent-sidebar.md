# Agent Sidebar Sketch

Status: Phase 1 UX contract artifact (`1.UX.4`)

## Phase Boundary

Sidebar location and interaction model are defined in Phase 1. Production agent observability lands in Phase 2.

## Layout

```text
+----------------------------+
| Agents                     |
| idle                       |
+----------------------------+
| Active                     |
| Claude Code                |
| waiting for approval       |
+----------------------------+
| Command                    |
| cargo test                 |
+----------------------------+
| Files touched              |
| crates/.../lib.rs          |
| docs/.../plan.md           |
+----------------------------+
| Approvals                  |
| [Allow] [Deny]             |
+----------------------------+
| [Stop] [Kill] [Restart]    |
| [Transcript] [Export]      |
+----------------------------+
```

## States

| State | Display |
|---|---|
| No agent support active | Collapsed rail with disabled icon |
| Agent running | Status, command, elapsed time |
| Waiting approval | Badge and approval queue section |
| Stopped | Last exit summary and restart action |
| Failed | Error summary and export diagnostics |

## Contract

1. Sidebar consumes read models from `bongterm-agents`, never raw process handles.
2. Lifecycle buttons route through process-control APIs when implemented.
3. Approval labels must show enforcement level explicitly.
4. Resource strip links to resource dashboard attribution row.

## Acceptance

1. Phase 1 can ship with sidebar collapsed and disabled without re-layout later.
2. Phase 2 can populate the sidebar without changing main window regions.
