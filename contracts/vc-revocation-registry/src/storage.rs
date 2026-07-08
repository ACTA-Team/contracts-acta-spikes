//! Storage layout and helpers.
//! Instance storage  → admin (singleton).
//! Persistent storage → per-VC revocation records (keyed by BytesN<32>).

use soroban_sdk::{contracttype, Address, BytesN, Env, Symbol};

const INSTANCE_TTL_THRESHOLD: u32 = 518_400;
const INSTANCE_TTL_EXTEND_TO: u32 = 3_110_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 3_110_400;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Revoked(BytesN<32>),
}

/// On-chain record for a revoked VC.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevocationRecord {
    pub issuer: Address,
    pub revoked_at: u64,
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
    e.storage().persistent().has(&DataKey::Revoked(vc_id.clone()))
}

pub fn read_revocation(e: &Env, vc_id: &BytesN<32>) -> Option<RevocationRecord> {
    e.storage().persistent().get(&DataKey::Revoked(vc_id.clone()))
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
