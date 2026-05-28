# First-Launch Onboarding Flow

Status: Phase 1 UX contract artifact (`1.UX.7`)

## Flow

```text
Welcome
  -> Shell
  -> Appearance
  -> Shell Integration
  -> Agents Detected
  -> Privacy and Storage
  -> Resource Budgets
  -> Finish
```

## Steps

| Step | Required choices |
|---|---|
| Welcome | Explain BongTerm as terminal cockpit in one screen |
| Shell | Pick default shell; detect PowerShell, CMD, WSL, Git Bash |
| Appearance | Theme, contrast, font size |
| Shell Integration | Enable/disable OSC integration; default on when supported |
| Agents Detected | Show Claude Code/Codex CLI detection, disabled if absent |
| Privacy and Storage | Telemetry off by default; explain local SQLite/chunks |
| Resource Budgets | Show default caps and dashboard access |
| Finish | Open first terminal tab |

## Defaults

1. Default shell: PowerShell if found.
2. Telemetry: off.
3. Shell integration: enabled for supported shells.
4. Agents: detect only; no auto-run.
5. MCP: no startup in Phase 1 onboarding.

## Acceptance

1. User can finish without optional integrations.
2. Missing shell or agent produces disabled state, not error.
3. Onboarding writes settings via `bongterm-settings`, not ad hoc files.
