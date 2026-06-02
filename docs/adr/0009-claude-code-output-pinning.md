# ADR-0009: Claude Code Output Pinning

Status: Accepted

Decision: Phase 5 pins Claude Code non-interactive reliability checks to the last three observed versions and treats output shape drift as a compatibility warning, not silent success.

Reason: MVP-0 replay and explanation depend on stable transcript extraction.
