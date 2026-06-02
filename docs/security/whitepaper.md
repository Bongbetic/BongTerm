# BongTerm Security Whitepaper

BongTerm uses ConPTY for terminal I/O, Windows JobObjects for process caps, DPAPI/CredMan for secrets, and explicit policy evaluation for dangerous commands.

MVP-0 forbids DLL injection, hidden-console scraping, undocumented syscalls, process hollowing, kernel drivers, and global keyboard hooks. Diagnostic export requires redaction preview and telemetry is off by default.
