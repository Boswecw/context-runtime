//! In-process bundle store for C1. No durability yet — DataForge-Local
//! persistence + replay is a later slice. The store is the authority for which
//! refs are admitted in a bundle, and fails closed on anything else.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::assemble::{AssembledBundle, StoredPayload};
use crate::error::{ContextError, Result};

#[derive(Clone, Default)]
pub struct BundleStore {
    inner: Arc<RwLock<HashMap<String, AssembledBundle>>>,
}

impl BundleStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a bundle, returning its context_bundle_id.
    pub fn put(&self, bundle: AssembledBundle) -> String {
        let id = bundle.manifest.context_bundle_id.clone();
        self.inner
            .write()
            .expect("bundle store poisoned")
            .insert(id.clone(), bundle);
        id
    }

    pub fn get(&self, bundle_id: &str) -> Option<AssembledBundle> {
        self.inner
            .read()
            .expect("bundle store poisoned")
            .get(bundle_id)
            .cloned()
    }

    /// Fetch one payload, fail-closed if the bundle is unknown or the ref is not
    /// in that bundle's admitted inventory (the deferred forgeHQ adapter boundary).
    pub fn get_payload(&self, bundle_id: &str, payload_ref: &str) -> Result<StoredPayload> {
        let guard = self.inner.read().expect("bundle store poisoned");
        let bundle = guard
            .get(bundle_id)
            .ok_or_else(|| ContextError::BundleNotFound(bundle_id.to_string()))?;

        if !bundle.payload_refs.iter().any(|r| r == payload_ref) {
            return Err(ContextError::RefNotAdmitted(payload_ref.to_string()));
        }

        bundle
            .payloads
            .get(payload_ref)
            .cloned()
            .ok_or_else(|| ContextError::RefNotAdmitted(payload_ref.to_string()))
    }
}
