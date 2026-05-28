# UX Contract

**Status:** Required before Phase 1 implementation (spec §9)

These 10 artifacts must exist under `docs/ux/` before any Phase 1 UI code is written.  
Low fidelity (ASCII sketches, bullet lists) is acceptable. The goal is a shared written reference, not a polished design.

## Required artifacts

| # | File | Content |
|---|---|---|
| 1 | `main-window-layout.md` | Main window layout sketch: title bar, tab strip, pane grid, status bar, sidebar positions |
| 2 | `command-palette.md` | Command palette behavior: trigger keys, filtering, categories, Cmd-K entry, preview |
| 3 | `pane-tab-model.md` | Pane + tab model sketch: split h/v, resize, focus cycle, tab drag, pane close |
| 4 | `agent-sidebar.md` | Agent sidebar sketch: agent list, status indicators, lifecycle controls, transcript scroll |
| 5 | `resource-dashboard.md` | Resource dashboard sketch: CPU/RAM/IO per process, VRAM gauge, process tree |
| 6 | `error-recovery-screen.md` | Error + recovery screen sketch: crash banner, safe mode prompt, restore dialog |
| 7 | `onboarding-flow.md` | First-launch onboarding flow: shell detection, profile setup, shortcut cheat sheet |
| 8 | `keyboard-shortcuts.md` | Keyboard shortcut table: all default bindings, categories, conflict rules |
| 9 | `notification-taxonomy.md` | Notification taxonomy: toast types, urgency levels, persistence, dismissal |
| 10 | `design-tokens.md` | Design tokens: typography (families, sizes), spacing scale, border radii, motion curves, color palette, semantic status colors (danger, warning, success, info, focus ring), high-contrast overrides, reduced-motion rules |

## Status

All 10 UX contract artifacts exist. Treat them as low-fidelity contracts; update them before changing Phase 1 UI behavior.
