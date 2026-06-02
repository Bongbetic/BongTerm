//! Credential-manager-shaped backend for DPAPI-wrapped secret blobs.
//!
//! Unit tests use `InMemoryBackend`; production wiring can swap this backend in.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::dpapi;
use crate::vault::VaultBackend;

static STORE: LazyLock<Mutex<HashMap<String, Vec<u8>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const TARGET_PREFIX: &str = "BongTerm:";

/// Backend surface for Windows secret persistence.
pub struct CredManBackend;

impl VaultBackend for CredManBackend {
    fn fetch(&self, name: &str) -> Option<Vec<u8>> {
        let key = format!("{TARGET_PREFIX}{name}");
        let blob = STORE.lock().unwrap().get(&key).cloned()?;
        dpapi::unprotect(&blob).ok()
    }

    fn put(&self, name: &str, plaintext: &[u8]) {
        let key = format!("{TARGET_PREFIX}{name}");
        if let Ok(blob) = dpapi::protect(plaintext) {
            STORE.lock().unwrap().insert(key, blob);
        }
    }
}
