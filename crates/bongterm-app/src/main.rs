//! `BongTerm` application binary entrypoint.

fn main() -> bongterm_ui::ShellResult {
    bongterm_diagnostics::install_panic_hook();
    bongterm_ui::run_shell()
}
