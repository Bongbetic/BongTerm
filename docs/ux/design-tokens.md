# Design Tokens

Status: Phase 1 UX contract artifact (`1.UX.10`)

## Typography

| Token | Value |
|---|---|
| `font.terminal` | Cascadia Mono, fallback monospace |
| `font.ui` | Segoe UI Variable, fallback sans-serif |
| `size.xs` | 11 |
| `size.sm` | 12 |
| `size.md` | 14 |
| `size.lg` | 16 |
| `size.xl` | 20 |

## Spacing and Radius

| Token | Value |
|---|---:|
| `space.1` | 4 |
| `space.2` | 8 |
| `space.3` | 12 |
| `space.4` | 16 |
| `space.5` | 24 |
| `space.6` | 32 |
| `radius.none` | 0 |
| `radius.sm` | 4 |
| `radius.md` | 8 |

## Motion

| Token | Value |
|---|---:|
| `motion.instant` | 0 ms |
| `motion.fast` | 120 ms |
| `motion.normal` | 240 ms |

Reduced-motion mode uses `motion.instant` for all nonessential transitions.

## Color Roles

| Token | Purpose |
|---|---|
| `terminal.bg` | Terminal background |
| `terminal.fg` | Terminal text |
| `terminal.selection` | Selected text |
| `terminal.cursor` | Cursor |
| `surface.app` | App chrome |
| `surface.panel` | Side panels |
| `border.default` | Panel and toolbar borders |
| `status.success` | Success/healthy |
| `status.warn` | Warning/degraded |
| `status.danger` | Dangerous/destructive |
| `status.info` | Informational |
| `focus.ring` | Keyboard focus |
| `production.danger` | Production safety mode |

## Accessibility

1. Focus ring must be visible on every actionable control.
2. High contrast maps semantic colors to OS high-contrast equivalents where available.
3. Status must not rely on color alone; labels remain visible.
4. Terminal color themes cannot override safety/danger tokens.

## Acceptance

1. Iced UI can derive spacing, radius, color, and type from these names.
2. Terminal rendering can keep its own theme while UI safety colors remain stable.
