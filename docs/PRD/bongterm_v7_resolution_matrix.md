# BongTerm v7 — Critical Analysis Resolution Matrix

This matrix maps every issue in the critical analysis to a PRD-level resolution.

## A. Scope Blockers

| ID | Critique | Resolution in v7 | Status |
|---|---|---|---|
| B1 | Custom parser + renderer + scrollback + benchmark harness in Phase 0 creates 1–2 year pre-user risk. | MVP-0 is reuse-first. Evaluate `vte`, `termwiz`, WezTerm core, Alacritty-derived components, or Windows Terminal interop. Custom terminal core only after failed spike. | Resolved |
| B2 | No team/budget/timeline assumptions. | Added team profiles: solo, 3–5 engineer, 6–10+ specialist team. Scope changes by team profile. | Resolved |
| B3 | Shared `bongterm-mcp-host.exe` is its own product. | Deferred shared MCP host pool to v1.1+. MVP-0 uses simple one-server-process supervision with resource caps and no pooling. | Resolved |
| B4 | Worktree attribution is brittle on Windows. | Moved full worktree orchestration to v1. Attribution is confidence-tagged: `git-confirmed`, `process-associated`, `watcher-associated`, `mixed`, `unknown`. Added edge-case tests. | Resolved |
| B5 | UIA provider over GPU grid is large specialist work. | Reduced MVP accessibility bar to Narrator active text, scrollback, blocks, tabs/panes, main controls. Rich UIA deferred or inherited through reused components. | Resolved |
| B6 | OS support matrix too broad. | Required: Windows 11 24H2. Best effort: Windows 11 23H2. Post-MVP: Windows 10, Server, RDP certification, WSL2 mirrored networking. | Resolved |
| B7 | Seven agent profiles in MVP create maintenance treadmill. | Built-in MVP-0 agents limited to Claude Code and Codex CLI. Others are community/import profiles. | Resolved |
| B8 | Raw Win32 custom UI controls create DPI/IME/accessibility risks. | Non-hot-path UI may use WinUI 3 or Slint. Terminal hot path remains native and isolated. | Resolved |

## B. Product Gaps

| Gap | Resolution |
|---|---|
| No UX wireframes/design system | Added design-token requirements and UI surfaces. Wireframes are a required design deliverable before implementation. |
| No onboarding | Added first-launch onboarding flow. |
| No empty/error/loading states | Added required state taxonomy for every major surface. |
| No design tokens | Added typography, spacing, radius, motion, color, semantic state, danger, focus, high-contrast, and reduced-motion tokens. |
| No keyboard shortcut map | Added minimum shortcut map. |
| No notification taxonomy | Added toast/sidebar/modal/inline notification rules. |
| No marketing/distribution plan | Added landing-page and distribution requirements. |
| Business model deferred | Added recommended open-core/pro/team split. |

## C. Engineering Gaps

| Gap | Resolution |
|---|---|
| Reference hardware undefined | Added Windows 11 24H2, Ryzen 7 7840U/Core Ultra 5-class, 16 GB RAM, integrated GPU, NVMe reference. |
| Plugin SDK unspecified | Added Tier 1 WASM and Tier 2 out-of-process plugin contracts. |
| AgentAdapter SDK unspecified | Added `AgentAdapter` trait and capability reporting model. |
| CI/CD vague | Added PR, nightly, channel, release, and performance gates. |
| Update-channel UX vague | Added Dev, Canary, Beta, Stable release channels. |
| Crash recovery UX missing | Added recovery screen, restore/discard/export behavior, culprit attribution. |
| Telemetry opt-in missing | Added off-by-default telemetry and redaction-preview export flow. |
| Localization unspecified | Added externalization requirement implicitly through UI/design readiness; full localization remains post-MVP unless business requires. |

## D. Developer-UX Additions

| Feature | v7 Placement |
|---|---|
| Cmd-K inline NL→command | MVP-0 |
| Failed-command explainer | MVP-0 |
| Background job notifications | MVP-0 |
| Snippet library with parameters | MVP-0 |
| Smart history filters | MVP-0 |
| Drag/drop file attachment | v1 candidate |
| Inline clickable errors | MVP-0 common patterns |
| HTTP/REST pane | v1.1 |
| DB query pane | v1.1 |
| Devcontainer runner | v1 |
| Branch graph/worktree overlay | v1 |
| Agent task pipeline | v1.1 |
| Cross-shell translator | v1 |
| Replay editor | v1 |
| Runbook/notebook mode | post-v1 |
| Clipboard history with redaction | post-v1 |
| PR comment browser | post-v1 |
| Keybinding role profiles | post-v1 |
| HTTP fetch + jq templates | post-v1 |

## E. Technical Recommendations

| Recommendation | v7 Handling |
|---|---|
| Reuse vte/termwiz/WezTerm/Alacritty components | Accepted for MVP-0 architecture. |
| WGPU instead of raw D3D-first | Accepted as preferred renderer abstraction with Direct3D backend. |
| WinUI 3/Slint for non-hot UI | Accepted. |
| Defer MCP pool | Accepted. |
| Limit agents to Claude/Codex | Accepted. |
| Add Cmd-K | Accepted. |
| Define dogfood gate | Accepted. |
| Pin reference hardware | Accepted. |
| Add telemetry redaction preview | Accepted. |

## F. Risks Added

| Risk | v7 Mitigation |
|---|---|
| Third-party CLI branding/licensing | Detect user-installed CLIs; do not bundle without legal review; avoid implied endorsement. |
| SmartScreen warnings | Code signing, reputation warm-up, checksum docs, enterprise deployment notes. |
| EDR false positives | No injection/hooks; security whitepaper; Defender + EDR validation plan. |
| WSL2 networking changes mid-session | Basic WSL2 only in MVP; mirrored networking/deep port handling post-MVP. |
| Prompt framework collisions | Shell compatibility matrix and confidence labels. |
| BongTerm trademark risk | USPTO/EUIPO/India/domain/package search before public launch. |
| Open-source maintenance burden | Supported vs community profile distinction, test fixtures, stale-profile policy. |

## G. Final Disposition

The critical analysis is not treated as a reason to cancel BongTerm. It is treated as a reason to narrow BongTerm.

The corrected product is:

> BongTerm: a Windows-first terminal for developers who want AI-agent workflows with command clarity, process accountability, and strict local control.
