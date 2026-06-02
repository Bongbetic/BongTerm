# ADR-0012: EDR-Friendly Process Tree

Status: Accepted

Decision: BongTerm supervision uses supported Windows APIs only and validates source/process trees with `xtask forbidden-abstraction`.

Reason: EDR trust is a release requirement; forbidden techniques are non-goals.
