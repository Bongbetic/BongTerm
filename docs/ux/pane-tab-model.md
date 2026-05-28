# Pane and Tab Model Sketch

Status: Phase 1 UX contract artifact (`1.UX.3`)

## Model

```text
Window
  Tab[]
    PaneTree
      Pane
      Split { axis: Horizontal | Vertical, ratio, first, second }
```

## Sketch

```text
[tab: repo pwsh] [tab: logs] [+]

+----------------------------------+-----------------------------+
| pwsh | C:\repo | High            | cargo test | C:\repo | Low  |
|                                  |                             |
| active prompt                    | test output                 |
|                                  |                             |
+----------------------------------+-----------------------------+
```

## Pane Chrome

Pane title bar shows:

| Field | Source |
|---|---|
| Process name | PTY/session metadata |
| CWD | shell integration or fallback session cwd |
| Confidence badge | `bongterm-blocks` confidence model |
| Focus ring | mux focus state |

## Behavior

1. New tab starts one pane with default shell.
2. Horizontal split creates left/right panes.
3. Vertical split creates top/bottom panes.
4. Resize handles adjust ratios; no pane below minimum terminal geometry.
5. Focus cycles in visual order.
6. Zoom/maximize hides sibling panes inside same tab, not other tabs.
7. Closing last pane closes tab after confirmation only if process still alive.

## Acceptance

1. Pane layout can serialize as a tree for restore.
2. Renderer receives per-pane surface snapshots, not mux internals.
3. Focus state is visually clear in high-contrast mode.
