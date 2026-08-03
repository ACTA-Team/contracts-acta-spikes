//! Storage layout and helpers.
//! Instance storage  → admin (global config, low-frequency reads).
//! Persistent storage → per-credential revocation records (long-lived, keyed by issuer + credential_id).
//!
//! Admin helpers and TTL constants are re-exported from `registry_core`.
//! Contract-specific error codes start at 10 (1–9 are shared in `CommonError`).

use registry_core;
use soroban_sdk::{contracttype, Address, Bytes, Env};

/// Storage keys separated by role (explicit role isolation).
///
/// Note: `Admin` is provided by `registry_core::DataKey::Admin` and is
/// NOT duplicated here. This enum only adds contract-specific keys.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Revocation record per (issuer, credential_id) pair (persistent storage).
    /// Composite key: (issuer Address, credential_id Bytes).
    Revocation(Address, Bytes),
}

/// On-chain marker indicating a revoked credential.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevocationRecord {
    /// Timestamp when the credential was revoked (ledger close time).
    pub revoked_at: u64,
}

// --- Admin (instance) — delegate to registry-core ---

pub use registry_core::{extend_instance_ttl, has_admin, read_admin, write_admin};

// --- Revocation records (persistent) ---

/// Check if a credential is revoked.
pub fn has_revocation(e: &Env, issuer: &Address, credential_id: &Bytes) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Revocation(issuer.clone(), credential_id.clone()))
}

/// Read a revocation record.
pub fn read_revocation(e: &Env, issuer: &Address, credential_id: &Bytes) -> Option<RevocationRecord> {
    e.storage()
        .persistent()
        .get(&DataKey::Revocation(issuer.clone(), credential_id.clone()))
}

/// Write a revocation record and extend its TTL.
pub fn write_revocation(
    e: &Env,
    issuer: &Address,
    credential_id: &Bytes,
    record: &RevocationRecord,
) {
    let key = DataKey::Revocation(issuer.clone(), credential_id.clone());
    e.storage().persistent().set(&key, record);
    registry_core::extend_persistent_ttl(e, &key);
}

/// Remove a revocation record (unrevoke).
pub fn remove_revocation(e: &Env, issuer: &Address, credential_id: &Bytes) {
    e.storage()
        .persistent()
        .remove(&DataKey::Revocation(issuer.clone(), credential_id.clone()));
}
