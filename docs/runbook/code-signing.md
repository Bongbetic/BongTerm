# Code Signing Runbook

**Status:** Phase 5 release-prep runbook

## Overview

BongTerm release binaries (MSIX) are signed with an OV (Organization Validated) code-signing certificate.  
EV (Extended Validation) certificate evaluation is deferred to post-`0.1.x` via ADR.

## Steps (Phase 5)

1. Obtain OV certificate from a CA that issues Windows code-signing certs (DigiCert, Sectigo, etc.).
2. Import cert into `CERT:\CurrentUser\My` on the signing machine.
3. Wire thumbprint into `cargo xtask package-msix` via environment variable `BONGT_SIGN_THUMBPRINT`.
4. Run `cargo xtask package-msix` on the release machine (never in sandbox CI).
5. Verify signed MSIX via `Get-AuthenticodeSignature`.

## SmartScreen

See `docs/runbook/smartscreen.md` for warm-up plan after first public release.

## Secrets

The signing certificate private key is stored in Windows Certificate Store (not in git, not in `.env`).  
See `docs/adr/0014-ev-cert-evaluation.md` for the OV-first / EV-later decision.
