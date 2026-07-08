//! Storage layout and helpers.
//! Instance storage  → admin (global config).
//! Persistent storage → per-schema records (keyed by BytesN<32>).

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, Symbol};

const INSTANCE_TTL_THRESHOLD: u32 = 518_400;
const INSTANCE_TTL_EXTEND_TO: u32 = 3_110_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 3_110_400;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Schema(BytesN<32>),
}

/// On-chain record for a registered schema.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaRecord {
    pub name: Symbol,
    pub version: u32,
    pub author: Address,
    pub content_hash: BytesN<32>,
    pub uri: Option<Bytes>,
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
    e.storage().persistent().has(&DataKey::Schema(schema_id.clone()))
}

pub fn read_schema(e: &Env, schema_id: &BytesN<32>) -> Option<SchemaRecord> {
    e.storage().persistent().get(&DataKey::Schema(schema_id.clone()))
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
