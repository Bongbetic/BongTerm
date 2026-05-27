# Fuzzing Runbook

## Toolchain policy

Fuzzing uses `cargo-fuzz`, which requires nightly Rust. This is the **only** permitted use of nightly in the BongTerm build system.

**Rules (binding — spec §2.3):**
1. The nightly toolchain is pinned in `tools/xtask/fuzz/rust-toolchain.toml` (separate from the workspace `rust-toolchain.toml` which pins stable 1.95).
2. The nightly toolchain version is bumped via ADR only — document the bump reason and new pin.
3. Fuzzing jobs run in isolation (`tools/xtask/fuzz/`) and never produce release artifacts.
4. CI fuzz jobs run nightly (separate workflow), not on every PR.
5. Fuzz corpora live in `tests/fixtures/fuzz_corpora/`.

## Targets (wired in Phase 5.C.1)

| Target | Crate | What it fuzzes |
|---|---|---|
| `fuzz_vt_parser` | `bongterm-term` | VT/ANSI/OSC parser over arbitrary byte streams |
| `fuzz_osc_consumer` | `bongterm-blocks` | OSC sequence consumer |
| `fuzz_settings_json5` | `bongterm-settings` | JSON5 settings deserialization |
| `fuzz_redactor` | `bongterm-security` | Redaction regex corpus |

## Bumping the nightly pin

1. Open `tools/xtask/fuzz/rust-toolchain.toml`.
2. Update the `channel` field to the new nightly date.
3. Run all fuzz targets locally for at least 60 seconds each.
4. Open an ADR entry in `docs/adr/` documenting the bump reason.
5. Commit with message: `chore(fuzz): bump nightly toolchain to <date> — <reason>`.

## Running fuzz locally

```powershell
# Install cargo-fuzz against the pinned nightly
cd tools/xtask/fuzz
cargo +nightly install cargo-fuzz --locked

# Run a target for N seconds
cargo +nightly fuzz run fuzz_vt_parser -- -max_total_time=60
```

## Corpus management

New crash inputs go in `tests/fixtures/fuzz_corpora/<target>/`. Commit them with the fix.
