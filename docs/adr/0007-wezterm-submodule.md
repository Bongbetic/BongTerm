# ADR-007: WezTerm Submodule API Stability Contract

**Status:** Pending — written at end of Spike S4  
**Date:** placeholder  
**Deciders:** Soubarna Karmakar

## Context

BongTerm vendors `wezterm-term`, `wezterm-mux`, and `termwiz` via a pinned git submodule at `vendor/wezterm/`. Upstream WezTerm evolves continuously. We need a policy for when to bump the pin and what constitutes an "API break" that triggers an ADR review.

Pinned tag at Phase 0 start: `20240203-110809-5046fc22`

## Spike S4 goal

Survey: `tools/spikes/s4-wezterm-api-stability/`  
Method: `git log --oneline <pin>..HEAD -- wezterm-term/src/ termwiz/src/ wezterm-mux/src/` on the upstream repo.  
Measure: API churn rate (public function/type additions, removals, signature changes) per calendar month.

## Questions to answer

1. How often do `wezterm-term`, `termwiz`, and `wezterm-mux` public APIs change in a breaking way?
2. What is the minimum surface area BongTerm consumes from each library?
3. What is the bump cadence recommendation (monthly, quarterly, on-demand)?

## Decision

*Pending S4 results. Update with:*
- *Consumed API surface list*
- *Bump cadence policy*
- *Break detection procedure (CI diff against last pin)*

## Consequences

Phase 1 task 1.B.3 (`WezTermAdapter::ingest_bytes` real wiring) depends on the API surface documented here.  
`xtask upstream-sync` uses the pinned tag from this ADR to generate delta reports.
