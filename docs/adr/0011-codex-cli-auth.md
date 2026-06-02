# ADR-0011: Codex CLI Auth Flow

Status: Accepted

Decision: Codex CLI auth is detected before launch and surfaced as unavailable when auth is missing. BongTerm does not automate credential entry.

Reason: Agent credentials are user-controlled secrets and must not be inferred or captured.
