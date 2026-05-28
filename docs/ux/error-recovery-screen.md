# Error and Recovery Screen Sketch

Status: Phase 1 UX contract artifact (`1.UX.6`)

## Banner

```text
+------------------------------------------------------------------------------+
| Recovery needed: renderer restarted after device loss. [Details] [Safe Mode] |
+------------------------------------------------------------------------------+
```

## Recovery Screen

```text
+------------------------------------------------------------+
| BongTerm recovered from a failure                          |
+------------------------------------------------------------+
| Affected items                                             |
| - Pane 2: shell still running, renderer snapshot restored   |
| - Resource sampler: stale for 4s                            |
+------------------------------------------------------------+
| Suspected culprit                                           |
| wgpu device removed during GPU switch                       |
+------------------------------------------------------------+
| [Restore] [Discard pane] [Export diagnostics] [Safe Mode]   |
+------------------------------------------------------------+
```

## Rules

1. Banner appears above tab strip; it never covers prompt input.
2. Recovery actions are per affected item when possible.
3. Export diagnostics always routes through redaction preview when implemented.
4. Safe Mode disables agents, MCP, custom renderer features, and nonessential startup restore.
5. User can continue typing in unaffected panes.

## Acceptance

1. Renderer, PTY, SQLite, settings parse, and sampler failures have distinct copy.
2. Recovery UI can explain partial degradation without forcing app exit.
3. Crash/recovery state has a stable route for future diagnostics implementation.
