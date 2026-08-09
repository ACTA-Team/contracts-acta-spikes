//! Storage layout and helpers.
//! No instance storage (there is no admin singleton).
//! Persistent storage → per-DID records (long-lived, keyed by BytesN<32>).

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env};

// TTL constants (~5 s ledger close): 518_400 ≈ 30 days, 3_110_400 ≈ 180 days.
const INSTANCE_TTL_THRESHOLD: u32 = 518_400;
const INSTANCE_TTL_EXTEND_TO: u32 = 3_110_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 3_110_400;

/// Storage keys separated by role (explicit role isolation).
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// DID record (per-DID-id, persistent storage).
    Did(BytesN<32>),
}

/// On-chain record for a registered DID.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DidRecord {
    /// Address that controls this DID and authorized its registration.
    pub controller: Address,
    /// Raw DID document bytes (e.g. UTF-8 JSON conforming to W3C DID Core).
    pub document: Bytes,
    /// Incremented on every successful `update`. Starts at 1 on registration.
    pub version: u32,
    /// Whether this DID has been deactivated. Non-destructive: record remains on-chain.
    pub deactivated: bool,
    /// Ledger timestamp at registration time (seconds since Unix epoch).
    pub created_at: u64,
    /// Ledger timestamp of the last mutation (update or deactivate).
    pub updated_at: u64,
}

// --- DID records (persistent) ---

pub fn has_did(e: &Env, did_id: &BytesN<32>) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Did(did_id.clone()))
}

pub fn read_did(e: &Env, did_id: &BytesN<32>) -> Option<DidRecord> {
    e.storage()
        .persistent()
        .get(&DataKey::Did(did_id.clone()))
}

pub fn write_did(e: &Env, did_id: &BytesN<32>, record: &DidRecord) {
    let key = DataKey::Did(did_id.clone());
    e.storage().persistent().set(&key, record);
    e.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

// --- TTL helpers ---

pub fn extend_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}
