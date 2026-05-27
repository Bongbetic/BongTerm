# WezTerm Local Patches

This directory holds explicit patch files applied on top of the pinned WezTerm
submodule. Every patch carries an ADR link explaining why it exists.

## Policy

1. Patches are applied at build time by `cargo xtask upstream-sync --apply-patches`.
   They are never committed into the submodule itself.
2. Every patch file MUST start with a header comment containing:
   - target file path within the submodule;
   - ADR reference (`docs/adr/NNNN-*.md`);
   - rationale (one paragraph).
3. Patches that cannot be re-applied cleanly after a submodule bump block the
   bump until an ADR review.
4. CI rejects PRs that modify `vendor/wezterm/` directly.

## Layout

```
vendor/wezterm.patches/
├─ README.md                  # this file
└─ <slug>.patch               # one patch per change; ordered alphabetically
```

## Pinned tag

`20240203-110809-5046fc22` — see `docs/adr/0007-wezterm-submodule.md` (ADR-005, written at end of Spike S4).

## Initializing the submodule

The submodule must be initialized before Phase 1 compilation:

```powershell
git submodule update --init --recursive
cd vendor/wezterm
git checkout 20240203-110809-5046fc22
cd ../..
```
