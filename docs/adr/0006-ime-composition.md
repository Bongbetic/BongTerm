# ADR-006: IME Composition on Terminal Surface

**Status:** Pending — written at end of Spike S3b  
**Date:** placeholder  
**Deciders:** Soubarna Karmakar

## Context

Windows IME (Input Method Editor) for CJK input (Chinese, Japanese, Korean) requires the terminal surface to support:
- Candidate window positioning relative to the cursor.
- Compose / cancel / commit events in the correct sequence.
- Surrogate pairs and grapheme cluster boundaries.

The implementation depends on the device integration shape chosen in ADR-005.

## Spike S3b goal

Harness: `tools/spikes/s3b-ime-composition/`  
Demonstrate: Chinese (Simplified) Pinyin input producing correct UTF-8 in the terminal buffer on the shape from ADR-005.

Test cases:
- Basic compose → commit
- Cancel mid-composition (Escape key)
- Surrogate pair character (e.g. emoji or CJK extension)
- Grapheme cluster with combining marks

## Decision

*Pending S3b results. Update with IME integration approach and any edge-case limitations.*

## Consequences

Phase 5.A.2 (IME wired to Phase 1 renderer shape) depends on this ADR.  
The acceptance gate §6.1 #21 (CJK input functional) is gated by this decision.
