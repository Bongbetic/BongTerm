# Resource Dashboard Sketch

Status: Phase 1 UX contract artifact (`1.UX.5`)

## Layout

```text
+------------------------------------------------+
| Resources                         live 1 Hz     |
+------------------------------------------------+
| Group        CPU   RSS    VRAM   IO/s   State   |
| BongT        4%    180MB  96MB   low    ok      |
| Shell        1%    70MB   -      low    ok      |
| conhost      0%    18MB   -      low    ok      |
| Agent        12%   620MB  -      med    warn    |
| MCP          0%    90MB   -      low    stale   |
+------------------------------------------------+
| Selected: Agent                                  |
| pid tree, caps, last sample, restart policy      |
+------------------------------------------------+
```

## Attribution Groups

`BongT`, `shell`, `conhost`, `agent`, `MCP`, `plugin-zero`.

## Data Rules

1. Measurements come from `bongterm-ledger`.
2. Enforcement decisions come from `bongterm-process-control` and policy code, not dashboard UI.
3. Dashboard labels stale/degraded data when sampler misses deadlines.
4. VRAM uses DXGI when available; fallback state is `unsupported`, not zero.

## Visual States

| State | Meaning |
|---|---|
| ok | Fresh sample within budget |
| warn | Near budget or sampler delayed |
| degraded | Partial measurement only |
| stale | Last sample outside freshness window |
| blocked | Launch/admission blocked by policy |

## Acceptance

1. Stale data cannot look live.
2. Per-process attribution maps to Phase 1 plan categories.
3. Dashboard can be opened from shortcut, palette, status warning, or resource strip.
