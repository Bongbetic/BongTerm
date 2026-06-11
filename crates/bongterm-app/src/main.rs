//! `BongTerm` application binary entrypoint.
//!
//! Runs the composed shell: `bongterm-ui` chrome around the live terminal
//! runtime.

fn main() -> iced::Result {
    bongterm_diagnostics::install_panic_hook();
    bongterm_app::shell_app::run()
}
