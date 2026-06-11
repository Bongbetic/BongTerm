//! `WindowsVault` — DPAPI/CredMan-backed `SecretStore`.
//! Plaintext exists only in memory at resolve time for authorized consumers.

use std::collections::HashMap;
use std::sync::Mutex;

use bongterm_secrets_api::{
    ConsumerId, ResolveError, SecretRef, SecretScope, SecretStore, SecretValue,
};

/// Pluggable storage backend. Production = DPAPI + Credential Manager.
pub trait VaultBackend: Send + Sync {
    fn fetch(&self, name: &str) -> Option<Vec<u8>>;
    fn put(&self, name: &str, plaintext: &[u8]);
}

/// In-memory backend for unit tests.
#[derive(Default)]
pub struct InMemoryBackend {
    map: Mutex<HashMap<String, Vec<u8>>>,
}

impl InMemoryBackend {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    /// # Panics
    /// Panics if internal mutex poisoned.
    pub fn store_raw(&self, name: &str, value: &[u8]) {
        self.map
            .lock()
            .unwrap()
            .insert(name.to_string(), value.to_vec());
    }
}

impl VaultBackend for InMemoryBackend {
    fn fetch(&self, name: &str) -> Option<Vec<u8>> {
        self.map.lock().unwrap().get(name).cloned()
    }

    fn put(&self, name: &str, plaintext: &[u8]) {
        self.map
            .lock()
            .unwrap()
            .insert(name.to_string(), plaintext.to_vec());
    }
}

/// Parser for `.env` content. Produces in-memory name/value pairs only.
pub struct EnvImport;

impl EnvImport {
    #[must_use]
    pub fn parse(content: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            let mut value = value.trim();
            if value.len() >= 2
                && ((value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\'')))
            {
                value = &value[1..value.len() - 1];
            }
            out.insert(key.to_string(), value.to_string());
        }
        out
    }
}

/// What agent/tool will receive at launch. References only, never values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchDisclosure {
    pub consumer: ConsumerId,
    pub secret_refs: Vec<String>,
}

/// Vault generic over backend. Authorization maps secret name to allowed consumers.
pub struct WindowsVault<B: VaultBackend> {
    backend: B,
    authz: HashMap<String, Vec<ConsumerId>>,
}

impl<B: VaultBackend> WindowsVault<B> {
    #[must_use]
    pub fn with_backend_and_authz(backend: B, authz: HashMap<String, Vec<ConsumerId>>) -> Self {
        Self { backend, authz }
    }

    fn is_authorized(&self, name: &str, consumer: &ConsumerId) -> bool {
        self.authz
            .get(name)
            .is_some_and(|allowed| allowed.contains(consumer))
    }

    pub fn import_dotenv(&self, content: &str) -> usize {
        let parsed = EnvImport::parse(content);
        for (name, value) in &parsed {
            self.backend.put(name, value.as_bytes());
        }
        parsed.len()
    }

    pub fn build_env_block(
        &self,
        spec: &[(String, String)],
        consumer: &ConsumerId,
    ) -> Result<Vec<(String, String)>, ResolveError> {
        let mut out = Vec::with_capacity(spec.len());
        for (name, raw) in spec {
            if let Some(secret_name) = raw
                .strip_prefix("${secret:")
                .and_then(|value| value.strip_suffix('}'))
            {
                let secret = SecretRef {
                    name: secret_name.to_string(),
                    scope: SecretScope::Agent,
                };
                let value = self.resolve(&secret, consumer)?;
                out.push((name.clone(), value.expose().to_string()));
            } else {
                out.push((name.clone(), raw.clone()));
            }
        }
        Ok(out)
    }

    #[allow(clippy::unused_self)]
    #[must_use]
    pub fn disclose(&self, spec: &[(String, String)], consumer: &ConsumerId) -> LaunchDisclosure {
        let secret_refs = spec
            .iter()
            .filter_map(|(_, raw)| {
                raw.strip_prefix("${secret:")
                    .and_then(|value| value.strip_suffix('}'))
                    .map(str::to_string)
            })
            .collect();
        LaunchDisclosure {
            consumer: consumer.clone(),
            secret_refs,
        }
    }
}

impl<B: VaultBackend> SecretStore for WindowsVault<B> {
    fn resolve(
        &self,
        secret: &SecretRef,
        consumer: &ConsumerId,
    ) -> Result<SecretValue, ResolveError> {
        let Some(bytes) = self.backend.fetch(&secret.name) else {
            return Err(ResolveError::Missing(secret.clone()));
        };
        if !self.is_authorized(&secret.name, consumer) {
            return Err(ResolveError::Unauthorized {
                secret: secret.clone(),
                consumer: consumer.clone(),
            });
        }
        let plaintext = String::from_utf8(bytes)
            .map_err(|err| ResolveError::Backend(format!("non-utf8 secret: {err}")))?;
        Ok(SecretValue::from_plaintext(plaintext))
    }

    fn exists(&self, secret: &SecretRef) -> bool {
        self.backend.fetch(&secret.name).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_secrets_api::{ResolveError, SecretScope, SecretStore};

    fn vault_with(name: &str, value: &str, authorized: &str) -> WindowsVault<InMemoryBackend> {
        let backend = InMemoryBackend::new();
        backend.store_raw(name, value.as_bytes());
        let mut authz = HashMap::new();
        authz.insert(name.to_string(), vec![ConsumerId(authorized.to_string())]);
        WindowsVault::with_backend_and_authz(backend, authz)
    }

    #[test]
    fn resolves_for_authorized_consumer() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let secret = SecretRef {
            name: "GITHUB_PAT".into(),
            scope: SecretScope::Agent,
        };
        let value = vault
            .resolve(&secret, &ConsumerId("agent:claude-code".into()))
            .unwrap();
        assert_eq!(value.expose(), "ghp_secretvalue");
    }

    #[test]
    fn unauthorized_consumer_is_rejected() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let secret = SecretRef {
            name: "GITHUB_PAT".into(),
            scope: SecretScope::Agent,
        };
        let err = vault
            .resolve(&secret, &ConsumerId("agent:other".into()))
            .unwrap_err();
        assert!(
            matches!(err, ResolveError::Unauthorized { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn missing_secret_fails_closed() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let secret = SecretRef {
            name: "NOT_THERE".into(),
            scope: SecretScope::Agent,
        };
        let err = vault
            .resolve(&secret, &ConsumerId("agent:claude-code".into()))
            .unwrap_err();
        assert!(matches!(err, ResolveError::Missing(_)), "got {err:?}");
    }

    #[test]
    fn imports_dotenv_into_vault_without_disk_plaintext() {
        let dotenv = "# comment\nGITHUB_PAT=ghp_fromfile\nEMPTY=\nQUOTED=\"with spaces\"\n";
        let parsed = EnvImport::parse(dotenv);
        assert_eq!(parsed.len(), 3);
        assert_eq!(
            parsed.get("GITHUB_PAT").map(String::as_str),
            Some("ghp_fromfile")
        );
        assert_eq!(
            parsed.get("QUOTED").map(String::as_str),
            Some("with spaces")
        );
        let backend = InMemoryBackend::new();
        let vault = WindowsVault::with_backend_and_authz(backend, HashMap::new());
        let count = vault.import_dotenv(dotenv);
        assert_eq!(count, 3);
        assert!(vault.exists(&SecretRef {
            name: "GITHUB_PAT".into(),
            scope: SecretScope::Workspace
        }));
    }

    #[test]
    fn build_env_block_resolves_references_late_and_fails_closed_on_missing() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let consumer = ConsumerId("agent:claude-code".into());
        let spec = vec![
            ("LOG".to_string(), "info".to_string()),
            ("TOKEN".to_string(), "${secret:GITHUB_PAT}".to_string()),
        ];
        let block = vault.build_env_block(&spec, &consumer).unwrap();
        assert_eq!(
            block
                .iter()
                .find(|(key, _)| key == "LOG")
                .map(|(_, value)| value.as_str()),
            Some("info")
        );
        assert_eq!(
            block
                .iter()
                .find(|(key, _)| key == "TOKEN")
                .map(|(_, value)| value.as_str()),
            Some("ghp_secretvalue")
        );

        let bad = vec![("TOKEN".to_string(), "${secret:NOPE}".to_string())];
        let err = vault.build_env_block(&bad, &consumer).unwrap_err();
        assert!(matches!(err, ResolveError::Missing(_)), "got {err:?}");
    }

    #[test]
    fn disclosure_lists_references_never_values() {
        let vault = vault_with("GITHUB_PAT", "ghp_secretvalue", "agent:claude-code");
        let consumer = ConsumerId("agent:claude-code".into());
        let spec = vec![
            ("LOG".to_string(), "info".to_string()),
            ("TOKEN".to_string(), "${secret:GITHUB_PAT}".to_string()),
        ];
        let disclosure = vault.disclose(&spec, &consumer);
        assert_eq!(disclosure.secret_refs, vec!["GITHUB_PAT".to_string()]);
        assert_eq!(disclosure.consumer, consumer);
        let dbg = format!("{disclosure:?}");
        assert!(
            !dbg.contains("ghp_secretvalue"),
            "disclosure leaked a value: {dbg}"
        );
    }
}
