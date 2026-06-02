# Phase 4 Threat-Model Coverage

Phase 4 implements the MVP-0 controls for PRD/spec threat priorities under the
security contract.

## Coverage Table

| Priority | Phase 4 control |
| --- | --- |
| Indirect prompt injection | All MCP/tool results remain untrusted input. Dangerous-command matcher routes destructive commands to `RequireApproval` or `Deny`; no auto-run. Per-agent MCP tool allowlist is default deny. |
| Supply-chain compromise | `npx -y` is rejected at MCP config import and by transport start. MCP config import validates schema and keeps argv explicit/version-pinned. |
| Secret exfiltration | `WindowsVault` resolves references late and in memory only. Env block never stores plaintext on disk or argv. Redactor covers persisted/exported text. Secret-leak corpus gate requires zero leaks. |
| Malicious VT/OSC escapes | Out of Phase 4 scope. Parser-owned; covered by earlier fuzz work and carried into Phase 5 parser hardening. |
| Malicious workspace config | Workspace trust defaults to `Untrusted`; risky config is honored only after explicit trust. |
| DoS / resource exhaustion | JobObject RSS + child-count caps, restart backoff ceiling to `Unhealthy`, and idle shutdown only with no attached agent. |

## Notes

- Context Optimizer is token-budget only. It does not reduce RSS or replace
  process governance.
- Dangerous-command policy is best-effort pattern matching with closed enums and
  explicit enforcement levels.
- Redaction is applied to persisted/exported/indexed/AI-context/diagnostic
  text, not the live visible terminal surface.
