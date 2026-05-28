# Main Window Layout Sketch

Status: Phase 1 UX contract artifact (`1.UX.1`)

## Intent

Define the stable first screen for BongTerm Phase 1: a usable terminal cockpit where terminal work remains primary, while agent/resource surfaces are present but not dominant.

## Desktop Layout

```text
+------------------------------------------------------------------------------+
| BongTerm  workspace: repo-name                         [min] [max] [close]   |
+------------------------------------------------------------------------------+
| [tab: PowerShell - repo] [tab: +]                         [search] [palette] |
+--------------------------+--------------------------------------+------------+
|                          |                                      |            |
| Agent sidebar            | Terminal surface                     | Resource   |
|                          |                                      | dashboard  |
| - active agent           | +----------------------------------+ |            |
| - current command        | | pane title: pwsh | repo | High   | | CPU  RSS   |
| - files touched          | +----------------------------------+ | VRAM IO    |
| - approvals              | |                                  | | stale      |
| - transcript link        | | shell output / prompt             | | degraded   |
|                          | | cursor + selection                | |            |
| collapsed width: 44 px   | |                                  | | collapsed  |
| expanded width: 280 px   | +----------------------------------+ | width:44px |
|                          |                                      | exp:320px  |
+--------------------------+--------------------------------------+------------+
| status: shell ready | cwd | branch | block confidence | warnings | resources |
+------------------------------------------------------------------------------+

Command palette overlay:

                 +----------------------------------------------+
                 | > command, file, history, setting, action     |
                 |----------------------------------------------|
                 | Reload Settings                              |
                 | Split Pane Right                             |
                 | Explain Last Failed Command                  |
                 | Open Resource Dashboard                      |
                 +----------------------------------------------+
```

## Regions

| Region | Phase 1 behavior |
|---|---|
| Title bar | Shows product name and active workspace/repo. Native window controls remain standard. |
| Tab strip | Shows active terminal tabs, new-tab affordance, and compact toolbar actions. |
| Left sidebar | Agent sidebar location. Collapsed by default until Phase 2 agent features are active; still reserves interaction model now. |
| Center surface | Primary terminal region. Must receive focus by default and keep typing latency priority. |
| Right sidebar | Resource dashboard location. Collapsed by default; opens via shortcut, palette, or warning state. |
| Status bar | Shows shell state, cwd, branch, command-block confidence, warning count, and resource state. |
| Palette overlay | Centered overlay above terminal surface; dismisses without mutating terminal state. |

## Layout Rules

1. Terminal surface is primary. Side panels never reduce a single pane below 80 columns unless user explicitly keeps panels open.
2. Panels are independent toggles. Agent and resource panels can both be collapsed, one open, or both open.
3. Palette overlays content without reflowing panes.
4. Status bar is always visible unless full-screen terminal mode is added later.
5. Focus order: terminal -> tab strip -> command palette -> sidebars -> status actions.
6. Error/recovery banner appears above the tab strip and pushes the terminal down; it never covers prompt input.

## Contract Boundaries

| UI element | Backing domain |
|---|---|
| Terminal surface | `bongterm-term` snapshot consumed by `bongterm-render` |
| Tab/pane strip | `bongterm-mux` pane/tab model |
| Agent sidebar | `bongterm-agents` read model, Phase 2 implementation |
| Resource dashboard | `bongterm-ledger` measurements |
| Settings overlay | `bongterm-settings` typed snapshot and validation errors |
| Recovery banner | `bongterm-diagnostics` and subsystem health events |

## Acceptance

1. Main window implementation can be derived without inventing additional regions.
2. Phase 1 single-pane tracer path fits in the center surface without requiring side panels.
3. Sidebar collapsed/expanded widths are fixed enough to avoid layout thrash.
4. Palette and recovery banner placement are unambiguous.
