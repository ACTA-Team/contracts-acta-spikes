//! Storage layout and helpers.
//! Instance storage  → admin (global config, low-frequency reads).
//! Persistent storage → per-credential revocation records (long-lived, keyed by issuer + credential_id).

use soroban_sdk::{contracttype, Address, Bytes, Env};

// TTL constants (~5 s ledger close): 518_400 ≈ 30 days, 3_110_400 ≈ 180 days.
const INSTANCE_TTL_THRESHOLD: u32 = 518_400;
const INSTANCE_TTL_EXTEND_TO: u32 = 3_110_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 3_110_400;

/// Storage keys separated by role (explicit role isolation).
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Global admin (singleton, instance storage)
    Admin,

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

// --- Admin (instance) ---

/// Check if admin has been initialized.
pub fn has_admin(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Admin)
}

/// Read the stored admin address.
pub fn read_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

/// Write the admin address to instance storage.
pub fn write_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

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

/// Write a revocation record.
pub fn write_revocation(
    e: &Env,
    issuer: &Address,
    credential_id: &Bytes,
    record: &RevocationRecord,
) {
    e.storage()
        .persistent()
        .set(&DataKey::Revocation(issuer.clone(), credential_id.clone()), record);
}

/// Remove a revocation record (unrevoke).
pub fn remove_revocation(e: &Env, issuer: &Address, credential_id: &Bytes) {
    e.storage()
        .persistent()
        .remove(&DataKey::Revocation(issuer.clone(), credential_id.clone()));
}

// --- TTL management ---

/// Extend instance storage TTL to prevent admin record expiration.
pub fn extend_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

/// Extend persistent storage TTL for a revocation record.
pub fn extend_revocation_ttl(e: &Env) {
    e.storage()
        .persistent()
        .extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}
