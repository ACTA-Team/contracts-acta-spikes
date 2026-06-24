//! Storage layout and helpers.
//! Instance storage  → admin (global config, low-frequency reads).
//! Persistent storage → per-schema records (long-lived, keyed by BytesN<32>).

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, Symbol};

// TTL constants (~5 s ledger close): 518_400 ≈ 30 days, 3_110_400 ≈ 180 days.
const INSTANCE_TTL_THRESHOLD: u32 = 518_400;
const INSTANCE_TTL_EXTEND_TO: u32 = 3_110_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 3_110_400;

/// Storage keys separated by role (explicit role isolation).
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Global admin (singleton, instance storage).
    Admin,
    /// Schema record (per-schema-id, persistent storage).
    Schema(BytesN<32>),
}

/// On-chain record for a registered VC schema.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaRecord {
    /// Address that registered the schema and authorized it.
    pub author: Address,
    /// Human-readable schema name.
    pub name: Symbol,
    /// Version string for this schema.
    pub version: Symbol,
    /// Raw schema definition bytes (e.g. UTF-8 JSON Schema).
    pub definition: Bytes,
    /// Whether this schema has been deprecated. Non-destructive: record remains on-chain.
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

pub fn has_schema(e: &Env, schema_id: &BytesN<32>) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Schema(schema_id.clone()))
}

pub fn read_schema(e: &Env, schema_id: &BytesN<32>) -> Option<SchemaRecord> {
    e.storage()
        .persistent()
        .get(&DataKey::Schema(schema_id.clone()))
}

pub fn write_schema(e: &Env, schema_id: &BytesN<32>, record: &SchemaRecord) {
    let key = DataKey::Schema(schema_id.clone());
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
