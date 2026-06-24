//! Storage layout and helpers.
//! Instance storage  → admin (global config, low-frequency reads).
//! Persistent storage → per-schema records (long-lived, keyed by schema id).

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

    /// Schema registry (per-id persistent storage)
    Schema(Bytes),
}

/// On-chain record for a registered VC schema.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaRecord {
    /// Address that registered the schema (and may update/deprecate it).
    pub author: Address,
    /// URI pointing to the schema document.
    pub uri: Bytes,
    /// Whether this schema has been deprecated.
    pub deprecated: bool,
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

// --- Schema records (persistent) ---

pub fn has_schema(e: &Env, id: &Bytes) -> bool {
    e.storage().persistent().has(&DataKey::Schema(id.clone()))
}

pub fn read_schema(e: &Env, id: &Bytes) -> Option<SchemaRecord> {
    e.storage().persistent().get(&DataKey::Schema(id.clone()))
}

pub fn write_schema(e: &Env, id: &Bytes, record: &SchemaRecord) {
    let key = DataKey::Schema(id.clone());
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
