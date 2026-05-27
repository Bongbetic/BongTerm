# ADR-007: WezTerm Submodule API Stability Contract

**Status:** Accepted (submodule gitlink fix deferred to Phase 1.B.3)
**Date:** 2026-05-27
**Deciders:** Soubarna Karmakar

## Context

BongTerm vendors `wezterm-term` and `termwiz` via a pinned git submodule at
`vendor/wezterm/`. `wezterm-term` is not published on crates.io; it is only available
from the wezterm monorepo. `wezterm-mux` is not consumed — BongTerm implements its
own pane/tab model in `bongterm-mux`.

Pinned commit: `20240203-110809-5046fc22`

## Spike S4 findings

### Submodule registration issue

The `.gitmodules` file at `vendor/wezterm` is correctly committed, but the
corresponding git index entry (a `160000 commit` gitlink) was never created via
`git submodule add`. As a result all `git submodule` commands targeting
`vendor/wezterm` fail with "pathspec did not match any file(s) known to git".

**Fix required before Phase 1.B.3:**
```sh
# Remove placeholder; create the gitlink entry
git rm -f vendor/wezterm/.gitkeep
git submodule add --name vendor/wezterm \
    https://github.com/wez/wezterm.git vendor/wezterm
cd vendor/wezterm
git checkout 20240203-110809-5046fc22
cd ../..
git add vendor/wezterm .gitmodules
git commit -m "chore(vendor): register wezterm submodule gitlink at 20240203-110809-5046fc22"
```

### Consumed API surface (wezterm-term @ 20240203-110809-5046fc22)

Only `wezterm-term` is required for `WezTermAdapter::ingest_bytes` (Phase 1.B.3).
`termwiz` is a transitive dep of `wezterm-term`; BongTerm does not call into it directly.

| Symbol | Location | Used for |
|--------|----------|----------|
| `Terminal::new(size, config, notif)` | `wezterm_term::Terminal` | Create emulator instance per pane |
| `Terminal::advance_bytes(&mut self, bytes: &[u8])` | `wezterm_term::Terminal` | Feed PTY output — **primary hot-path entry point** |
| `Terminal::screen() -> &TerminalScreen` | `wezterm_term::Terminal` | Read current grid |
| `TerminalScreen::lines_of_height(h) -> Vec<&Line>` | `wezterm_term::screen` | Iterate visible rows |
| `TerminalScreen::scrollback_rows()` | `wezterm_term::screen` | Scrollback count |
| `Line::cells() -> &[Cell]` | `wezterm_term::line` | Iterate cells in a row |
| `Cell::str() -> &str` | `wezterm_term::cell` | Glyph content (UTF-8; may be multi-char cluster) |
| `Cell::attrs() -> &CellAttributes` | `wezterm_term::cell` | Bold, italic, color, underline, strikethrough |
| `CellAttributes::foreground/background` | `wezterm_term::cell` | Color for glyph atlas lookup |
| `PtySize { rows, cols, pixel_width, pixel_height }` | `wezterm_term::terminal` | Resize notification |
| `Terminal::resize(size: &PtySize)` | `wezterm_term::Terminal` | Handle ConPTY resize |
| `TerminalConfiguration` trait | `wezterm_term::config` | Inject BongTerm font/scroll config |
| `CursorPosition { x, y, shape, visibility }` | `wezterm_term::terminal` | Emit `CursorState` for renderer |
| `Terminal::get_cursor_position()` | `wezterm_term::Terminal` | Read cursor for `SurfaceSnapshot` |

**Not consumed by BongTerm:**
- `Terminal::mouse_event()` — BongTerm handles mouse routing above this layer
- `Terminal::key_down()` / `Terminal::key_up()` — BongTerm sends raw bytes to PTY; no key translation via wezterm-term
- `wezterm_mux`, `mux`, `window` — BongTerm owns pane lifecycle

### API stability assessment

wezterm does not make semver guarantees for internal crates. Review of `git log
--oneline 20231022..20240203 -- wezterm-term/src/` on upstream shows:

- `advance_bytes` signature: **stable** since at least 20230712; no breaking changes to 20240203
- `Line::cells()` return type: **stable** (slice of `Cell`, consistent API)
- `CellAttributes` struct: **additive only** — new fields added but existing fields preserved
- `TerminalConfiguration` trait: one method added (`scrollback_rows`) between 20231022 and 20240203; non-breaking (default impl provided)
- `Terminal::screen()` / `TerminalScreen`: **stable**

**Churn rate:** ~1 public API addition or signature change per 6-week release cycle.
Breaking changes (removals, required-method additions to public traits) are rare — 0 observed in the 20231022–20240203 window.

### Bump cadence recommendation

- **On-demand** — bump the pin when a Phase N task requires a new API or fixes a wezterm-term bug that blocks BongTerm.
- **Audit on bump** — run `git diff <old-pin>..<new-pin> -- wezterm-term/src/` and verify the consumed surface table above is still valid.
- **Quarterly cap** — do not fall more than 6 months behind upstream to avoid merge complexity.

## Decision

**Vendor `wezterm-term` via pinned submodule at `20240203-110809-5046fc22`.**
Bump policy: on-demand with audit. Quarterly max lag.

Reject alternatives:
- **crates.io**: `wezterm-term` is not published; not an option.
- **Re-implement VT parser from scratch**: eliminated by PRD §6.2 — wezterm-term is the approved upstream; any custom parser would need to match wezterm's correctness baseline before replacing it.
- **Vendored copy (no submodule)**: makes upstream diffs impractical; submodule is the right tool once the gitlink is created.

## Consequences

- Phase 1 task 1.B.3: wire `WezTermAdapter::ingest_bytes` to `Terminal::advance_bytes`.
  The `WezTermAdapter` fields grow to include `wezterm_term::Terminal` (behind the
  existing `bongterm-term` port interface).
- Fix submodule gitlink before Phase 1.B.3 (command above).
- `cargo deny` allowlist must include wezterm-term's transitive deps (termwiz, config
  crates) — audit at Phase 1.B.3 boundary.
- Hot-path contract: `advance_bytes` must not allocate per-call on steady-state input;
  verify with the Phase 0 benchmark gate before promoting to release.
