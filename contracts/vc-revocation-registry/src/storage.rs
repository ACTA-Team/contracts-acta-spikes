//! Storage layout and helpers.
//! Instance storage → admin (global config, low-frequency reads).
//! Persistent storage → revocation records (long-lived, keyed by VC id).

use soroban_sdk::{contracttype, Address, BytesN, Env, Symbol};

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

    /// Revocation record (per-VC persistent storage)
    Revoked(BytesN<32>),
}

/// Revocation record stored for each revoked VC.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevocationRecord {
    /// The issuer who originally issued the VC.
    pub issuer: Address,
    /// Timestamp (ledger close time) when the VC was revoked.
    pub revoked_at: u64,
    /// Optional reason for revocation.
    pub reason: Option<Symbol>,
}

// --- Admin (instance) ---

pub fn has_admin(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Admin)
}

pub fn read_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn write_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

// --- Revocation records (persistent) ---

pub fn has_revocation(e: &Env, vc_id: &BytesN<32>) -> bool {
    let key = DataKey::Revoked(vc_id.clone());
    let has = e.storage().persistent().has(&key);
    if has {
        // Extend TTL when checking revocation status to prevent expiration
        e.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    }
    has
}

pub fn read_revocation(e: &Env, vc_id: &BytesN<32>) -> Option<RevocationRecord> {
    let key = DataKey::Revoked(vc_id.clone());
    let record = e.storage().persistent().get(&key);
    if record.is_some() {
        // Extend TTL when reading revocation to prevent expiration
        e.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    }
    record
}

pub fn write_revocation(e: &Env, vc_id: &BytesN<32>, record: &RevocationRecord) {
    let key = DataKey::Revoked(vc_id.clone());
    e.storage().persistent().set(&key, record);
    e.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

pub fn remove_revocation(e: &Env, vc_id: &BytesN<32>) {
    e.storage().persistent().remove(&DataKey::Revoked(vc_id.clone()));
}

// --- TTL helpers ---

pub fn extend_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}
