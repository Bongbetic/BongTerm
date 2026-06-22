# BongTerm

BongTerm is an experimental MVP-0 Windows terminal for developers who want AI-agent workflows with command clarity, process accountability, and strict local control.

## What It Is

- Resource-governed agent terminal for Windows-first development.
- No Electron in the terminal hot path.
- No cloud account required.
- Child-process resource dashboard for shells, agents, MCP servers, and background jobs.
- Cmd-K command generation with preview-only execution.
- Failed-command explanation from local command context.
- Claude and Codex support without bundling third-party CLIs.
- Privacy and local-first defaults; telemetry off by default.

## Status

`v0.1.0-mvp0` is not shipped yet. The repository is public and the 7 scheduled
remote nightlies gate is complete, but shipment is still blocked on visible MVP
UI real-state wiring, signed clean-VM installer proof, dogfood, legal/name
review, and final GitHub release verification.

## Install

See `INSTALL.md` once signed MSIX artifacts are published.

## Security

See `SECURITY.md`. Do not report vulnerabilities in public issues.

## Specs

- Product spec: `docs/PRD/bongterm_prd_v7.md`
- Engineering spec: `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md`
