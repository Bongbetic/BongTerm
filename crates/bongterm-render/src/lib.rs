//! bongterm-render
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.
//!
//! SCAFFOLD ONLY — product renderer wgpu+glyphon implementation begins only after
//! ADR-002/003/004a/004b are accepted (Wave 0 spikes).

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
