//! `BongTerm` secret references and trait surfaces.
//!
//! Spec §3.7 / §37. Every subsystem handling secrets depends on this crate.
//! Concrete storage (Windows Credential Manager / DPAPI) lives in
//! `bongterm-vault-windows` and is wired only by `bongterm-app`.

// deny (not forbid) allows the scoped #[allow(unsafe_code)] on the Drop impl below.
#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use std::fmt;

/// Where in the policy hierarchy a secret is scoped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretScope {
    Global,
    Workspace,
    Profile,
    Agent,
    Mcp,
}

/// Exposure class (spec §37.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExposureClass {
    NoSecretAccess,
    ReferenceVisibleOnly,
    BrokeredOperation,
    EphemeralScopedToken,
    ReadOnlyScopedCredential,
    RawEnvInjection,
}

/// Reference to a secret. Carries no value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SecretRef {
    pub name: String,
    pub scope: SecretScope,
}

/// Identity of a consumer authorized to receive a resolved secret.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ConsumerId(pub String);

/// A resolved plaintext value. Never `Display`s or `Debug`s the actual contents.
pub struct SecretValue(String);

impl SecretValue {
    #[must_use]
    pub fn from_plaintext(p: String) -> Self {
        Self(p)
    }

    /// Caller takes responsibility for redacting before any persistence/export.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<redacted>")
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretValue(<redacted>)")
    }
}

#[allow(unsafe_code)]
impl Drop for SecretValue {
    fn drop(&mut self) {
        // Zero plaintext on drop — best-effort; write_volatile resists compiler reordering.
        // SAFETY: writing zero bytes through the String's own allocation is sound; we own it.
        unsafe {
            let bytes = self.0.as_mut_vec();
            for b in bytes.iter_mut() {
                std::ptr::write_volatile(b, 0);
            }
        }
    }
}

/// Port through which `BongTerm` reads (never writes plain) secret values.
/// Implementations live in `bongterm-vault-windows`.
pub trait SecretStore: Send + Sync {
    /// Resolve a secret reference to a value, scoped to the given consumer.
    fn resolve(
        &self,
        secret: &SecretRef,
        consumer: &ConsumerId,
    ) -> Result<SecretValue, ResolveError>;

    /// Whether a secret reference exists (does NOT resolve the value).
    fn exists(&self, secret: &SecretRef) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("missing secret: {0:?}")]
    Missing(SecretRef),
    #[error("consumer {consumer:?} not authorized for {secret:?}")]
    Unauthorized {
        secret: SecretRef,
        consumer: ConsumerId,
    },
    #[error("vault backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_value_display_is_redacted() {
        let v = SecretValue::from_plaintext("hunter2".to_string());
        let shown = format!("{v}");
        assert_eq!(shown, "<redacted>");
    }

    #[test]
    fn secret_value_debug_is_redacted() {
        let v = SecretValue::from_plaintext("hunter2".to_string());
        let dbg = format!("{v:?}");
        assert!(dbg.contains("<redacted>"), "Debug must redact, got {dbg}");
        assert!(!dbg.contains("hunter2"), "Debug leaked plaintext!");
    }
}
