# Known Issues

- Git Bash, WSL, SSH, and PowerShell 7 shell-smoke coverage depends on tools installed on the test machine.
- Clean-VM MSIX install smoke requires Windows SDK `makeappx.exe` and signing material.
- SmartScreen reputation cannot be fully automated locally; use `docs/runbook/smartscreen.md`.
