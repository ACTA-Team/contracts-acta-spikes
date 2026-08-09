//! Storage layout and helpers.
//! Instance storage  → admin (global config, low-frequency reads).
//! Persistent storage → per-schema records (long-lived, keyed by BytesN<32>).
//!
//! Admin helpers and TTL constants are re-exported from `registry_core`.
//! Contract-specific error codes start at 10 (1–9 are shared in `CommonError`).

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, Symbol};

/// Storage keys separated by role (explicit role isolation).
///
/// Note: `Admin` is provided by `registry_core::DataKey::Admin` and is
/// NOT duplicated here. This enum only adds contract-specific keys.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
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

// --- Admin (instance) — delegate to registry-core ---

pub use registry_core::{extend_instance_ttl, has_admin, read_admin, write_admin};

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
    registry_core::extend_persistent_ttl(e, &key);
}
