# Notification Taxonomy

Status: Phase 1 UX contract artifact (`1.UX.9`)

## Channels

| Channel | Use |
|---|---|
| Toast | Background completion that does not need immediate action |
| Banner | Recoverable degraded state affecting current workspace |
| Sidebar badge | Agent/resource item needs attention |
| Inline confirmation | Dangerous command or local action confirmation |
| Modal | Launch-blocking, privacy, export, or destructive decision |

## Events

| Event | Channel |
|---|---|
| Background job done/failed | Toast, Phase 3 |
| Agent waiting for approval | Sidebar badge, Phase 2 |
| Resource budget exceeded | Dashboard warning; modal only if launch-blocking |
| Dangerous command detected | Inline confirmation, Phase 4 |
| MCP server crashed | Sidebar/dashboard badge, Phase 4 |
| Telemetry export requested | Modal with redaction preview, Phase 5 |
| Renderer recovered | Banner |
| Settings parse failed | Banner + settings overlay link |

## Severity

| Severity | Behavior |
|---|---|
| info | Nonblocking, auto-dismiss allowed |
| warn | Persistent until viewed or state clears |
| danger | Requires explicit action |
| blocked | Stops launch/action until resolved |

## Acceptance

1. Same event always maps to same channel unless severity escalates.
2. Notifications never obscure terminal prompt input.
3. Blocking states explain exact subsystem and next action.
