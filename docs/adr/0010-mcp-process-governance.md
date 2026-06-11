# ADR-0010: MCP Process Governance

- Status: Accepted
- Date: 2026-06-03

## Context

Phase 4 adds MCP support under BongTerm MVP-0 security and resource-governance
constraints. PRD v7 rejects a shared MCP host pool for MVP-0. The product must
keep one process per server/workspace, expose permissions visibly, reject
auto-install flows, and maintain explicit resource ceilings.

## Decision

For MVP-0, BongTerm governs MCP processes with these rules:

1. One process per server per workspace.
2. Windows JobObject caps apply at registration.
3. Default MCP RSS cap is 60 MB with child-process count caps.
4. Idle shutdown is allowed only when no active agent is attached.
5. Restart backoff schedule is 1s, 5s, 30s, then the server is marked
   `Unhealthy` and auto-restart stops.
6. MCP config import rejects `npx -y` auto-install commands.
7. Context Optimizer is token-budget governance only. It prunes exposed tool
   schema for agents; it does not claim any RSS reduction.

## Consequences

- MVP-0 stays within reuse-first scope and avoids pool/multiplex complexity.
- Resource limits and restart ceilings are explicit and testable.
- Supply-chain risk is reduced by rejecting auto-install at both config import
  and transport start.
- Future shared-pool work remains possible, but only after usage data and a new
  ADR.
