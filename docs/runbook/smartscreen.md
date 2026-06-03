# SmartScreen Runbook

**Status:** Phase 5 release-prep runbook

## Problem

New publishers with OV certificates trigger Windows SmartScreen "Unknown publisher" warnings on first run. This degrades Stage B dogfood UX.

## Warm-up plan (Phase 6.D.2)

1. Before public flip, distribute the signed MSIX to Stage B users via private channel.
2. Request Stage B users explicitly run the binary and click through SmartScreen (building reputation).
3. Submit to Microsoft SmartScreen reputation service once Stage B produces enough signed installs to show reproducible warning behavior.
4. Monitor SmartScreen block rate via opt-in diagnostics.

## Long-term

Evaluate EV certificate post-`0.1.x`. EV certificates bypass SmartScreen on first install.  
Decision captured in ADR when OV warm-up data is available.

## Executed Log

No Stage B installs yet. Execution blocked until signed dev-channel MSIX distribution.

## References

- https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/
