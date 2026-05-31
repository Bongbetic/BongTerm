//! `BongTerm` application binary entrypoint.
//!
//! Repointed to the terminal slice (`terminal_app`): a window running a real
//! shell. The richer `bongterm_ui` shell (tabs / palette / sidebar) is
//! temporarily bypassed here and folded back in as a follow-up — see
//! `SHIP-READINESS.md`.

fn main() -> iced::Result {
    bongterm_diagnostics::install_panic_hook();
    bongterm_app::terminal_app::run()
}
