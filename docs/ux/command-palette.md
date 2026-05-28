# Command Palette Behavior

Status: Phase 1 UX contract artifact (`1.UX.2`)

## Trigger

Default binding: `Ctrl+Shift+P`.

Palette opens centered over the terminal surface. Terminal input pauses while palette owns focus. `Esc` closes palette and restores terminal focus without side effects.

## Layout

```text
+------------------------------------------------+
| > split right                                  |
+------------------------------------------------+
| Layout      Split Pane Right              Alt+Shift+D |
| Layout      Split Pane Down               Alt+Shift+- |
| Terminal    New Tab                       Ctrl+Shift+T |
| Settings    Reload Settings               -           |
| History     Smart History                 Ctrl+R       |
+------------------------------------------------+
```

## Result Types

| Type | Examples |
|---|---|
| Terminal | New Tab, Close Pane, Find in Pane |
| Layout | Split Right, Split Down, Focus Next Pane |
| Settings | Reload Settings, Open Settings |
| History | Smart History, Rerun Block |
| Diagnostics | Export Diagnostics, Safe Mode |
| Developer UX | Cmd-K, Explain Last Failed Command |

## Filtering

1. Case-insensitive substring match across title, category, and aliases.
2. Prefix filters reserve future compatibility with smart history: `cwd:`, `branch:`, `agent:`, `exit:`, `time:`, `shell:`, `duration:`.
3. Empty query shows recent and common actions.
4. Dangerous actions require a secondary confirmation row, never immediate execution.

## Keyboard Model

| Key | Behavior |
|---|---|
| `Ctrl+Shift+P` | Open palette |
| `Esc` | Close palette |
| `Enter` | Run selected command |
| `Up` / `Down` | Move selection |
| `Tab` | Accept highlighted completion if present |
| `Ctrl+K` | Open Cmd-K entry when palette is closed |

## Acceptance

1. Palette behavior is deterministic without needing mouse input.
2. Filtering does not mutate terminal scrollback or prompt state.
3. `Reload Settings` is discoverable for Phase 1 settings work.
4. Cmd-K appears but may route to Phase 3 disabled state until implemented.
