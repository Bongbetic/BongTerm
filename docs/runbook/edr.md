# EDR / Defender Smoke

Use only supported user-mode primitives: ConPTY, JobObjects, DPAPI, DXGI, UIA, and signed MSIX packaging.

Release smoke:

1. Enable Defender real-time protection.
2. Install signed dev-channel MSIX.
3. Launch CMD, Windows PowerShell, PowerShell 7, one agent run, and one MCP server.
4. Run `cargo run -p xtask -- forbidden-abstraction`.
5. Confirm no Defender alert and no blocked process-tree event.
