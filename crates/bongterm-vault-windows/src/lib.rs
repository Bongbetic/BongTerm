//! bongterm-vault-windows — DPAPI / Credential Manager `SecretStore`.
//! See spec §1.2 ownership matrix + §37 secrets reference model.

#![cfg_attr(not(windows), forbid(unsafe_code))]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod vault;

#[cfg(windows)]
pub mod credman;
#[cfg(windows)]
pub mod dpapi;

#[cfg(windows)]
pub use credman::CredManBackend;
pub use vault::{EnvImport, InMemoryBackend, LaunchDisclosure, VaultBackend, WindowsVault};
