//! Conformance suite for [`bongterm_secrets_api::SecretStore`].

use bongterm_secrets_api::{ConsumerId, ResolveError, SecretRef, SecretScope, SecretStore};

/// Run happy-path conformance checks against any [`SecretStore`] implementation.
///
/// # Panics
///
/// Panics if any conformance assertion fails, indicating the implementation
/// does not satisfy the port contract.
pub fn run(store: &impl SecretStore) {
    let secret_ref = SecretRef {
        name: "CONFORMANCE_MISSING_SECRET".to_string(),
        scope: SecretScope::Global,
    };

    // exists() must not panic (result is deliberately ignored — it is
    // implementation-defined whether a test secret is present).
    let _ = store.exists(&secret_ref);

    // resolve() on a missing secret must return Err(ResolveError::Missing(_)).
    let consumer = ConsumerId("conformance-consumer".to_string());
    let result = store.resolve(&secret_ref, &consumer);
    assert!(
        matches!(result, Err(ResolveError::Missing(_))),
        "resolve() on a missing secret must return Err(ResolveError::Missing(_)), got: {result:?}"
    );
}
