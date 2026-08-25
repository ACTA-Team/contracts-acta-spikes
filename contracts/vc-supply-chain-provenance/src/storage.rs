//! Storage layout and helpers for vc-supply-chain-provenance.
//!
//! Admin helpers and TTL constants are re-exported from `registry_core`.

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env};

use crate::contract::Batch;

/// Contract storage keys.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Revocation registry contract address (instance).
    RevocationRegistry,
    /// Batch record keyed by id (persistent).
    Batch(BytesN<32>),
    /// Custody hop keyed by `(batch_id, index)` (persistent).
    Hop(BytesN<32>, u32),
    /// Attached credential id keyed by batch (persistent).
    Certificate(BytesN<32>),
}

pub use registry_core::{extend_instance_ttl, has_admin, read_admin, write_admin};

pub fn read_revocation_registry(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&DataKey::RevocationRegistry)
        .unwrap()
}

pub fn write_revocation_registry(e: &Env, registry: &Address) {
    e.storage()
        .instance()
        .set(&DataKey::RevocationRegistry, registry);
}

pub fn has_batch(e: &Env, batch_id: &BytesN<32>) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Batch(batch_id.clone()))
}

pub fn read_batch(e: &Env, batch_id: &BytesN<32>) -> Batch {
    e.storage()
        .persistent()
        .get(&DataKey::Batch(batch_id.clone()))
        .unwrap()
}

pub fn write_batch(e: &Env, batch_id: &BytesN<32>, batch: &Batch) {
    let key = DataKey::Batch(batch_id.clone());
    e.storage().persistent().set(&key, batch);
    registry_core::extend_persistent_ttl(e, &key);
}

pub fn has_hop(e: &Env, batch_id: &BytesN<32>, index: u32) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Hop(batch_id.clone(), index))
}

pub fn read_hop(e: &Env, batch_id: &BytesN<32>, index: u32) -> crate::contract::CustodyHop {
    e.storage()
        .persistent()
        .get(&DataKey::Hop(batch_id.clone(), index))
        .unwrap()
}

pub fn write_hop(e: &Env, batch_id: &BytesN<32>, index: u32, hop: &crate::contract::CustodyHop) {
    let key = DataKey::Hop(batch_id.clone(), index);
    e.storage().persistent().set(&key, hop);
    registry_core::extend_persistent_ttl(e, &key);
}

pub fn has_certificate(e: &Env, batch_id: &BytesN<32>) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Certificate(batch_id.clone()))
}

pub fn read_certificate(e: &Env, batch_id: &BytesN<32>) -> Bytes {
    e.storage()
        .persistent()
        .get(&DataKey::Certificate(batch_id.clone()))
        .unwrap()
}

pub fn write_certificate(e: &Env, batch_id: &BytesN<32>, credential_id: &Bytes) {
    let key = DataKey::Certificate(batch_id.clone());
    e.storage().persistent().set(&key, credential_id);
    registry_core::extend_persistent_ttl(e, &key);
}
