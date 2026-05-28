# Keyboard Shortcut Table

Status: Phase 1 UX contract artifact (`1.UX.8`)

## Defaults

| Action | Shortcut | Phase |
|---|---:|---|
| Command palette | `Ctrl+Shift+P` | 1 |
| Cmd-K | `Ctrl+K` | 3 |
| New tab | `Ctrl+Shift+T` | 1 |
| Split pane | `Alt+Shift+D` | 1 |
| Close pane | `Ctrl+Shift+W` | 1 |
| Find in pane | `Ctrl+F` | 1 |
| Smart history | `Ctrl+R` | 3 |
| Explain last failed command | `Ctrl+Shift+E` | 3 |
| Open resource dashboard | `Ctrl+Shift+R` | 1 |
| Attach context | `Ctrl+Shift+A` | 3 |
| Toggle background jobs | `Ctrl+Shift+J` | 3 |

## Conflict Rules

1. Terminal application shortcuts take priority only when palette/sidebar owns focus.
2. When terminal owns focus, shell-critical bindings pass through unless BongTerm binding is explicit and configurable.
3. User overrides live in settings JSON5.
4. Invalid or duplicate bindings fail validation and keep last-known-good settings active.

## Acceptance

1. Every default binding has one owner.
2. Phase 3 shortcuts may display disabled actions in Phase 1.
3. Shortcut table maps to settings schema names.
