# Known Issues

- Git Bash, WSL, SSH, and PowerShell 7 shell-smoke coverage depends on tools installed on the test machine.
- Clean-VM MSIX install smoke requires Windows SDK `makeappx.exe` and signing material.
- SmartScreen reputation cannot be fully automated locally; use `docs/runbook/smartscreen.md`.
- Phase 6 cannot exit until Stage A dogfood, Stage B, remote nightly proof, clean-VM signed install smoke, and public release publication complete.
- SECURITY.md still needs a real monitored disclosure inbox before repository public flip.
