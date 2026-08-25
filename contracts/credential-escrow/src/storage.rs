//! Storage layout and helpers for credential-escrow.
//!
//! Admin helpers and TTL constants are re-exported from `registry_core`.

use soroban_sdk::{contracttype, Address, Env};

use crate::contract::Escrow;

/// Contract storage keys.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// VC verifier contract address (instance).
    Verifier,
    /// Next escrow identifier (instance).
    NextEscrowId,
    /// Escrow record keyed by id (persistent).
    Escrow(u64),
}

pub use registry_core::{extend_instance_ttl, has_admin, read_admin, write_admin};

pub fn read_verifier(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Verifier).unwrap()
}

pub fn write_verifier(e: &Env, verifier: &Address) {
    e.storage().instance().set(&DataKey::Verifier, verifier);
}

pub fn read_next_escrow_id(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&DataKey::NextEscrowId)
        .unwrap_or(1)
}

pub fn write_next_escrow_id(e: &Env, next_id: u64) {
    e.storage().instance().set(&DataKey::NextEscrowId, &next_id);
}

pub fn has_escrow(e: &Env, escrow_id: u64) -> bool {
    e.storage().persistent().has(&DataKey::Escrow(escrow_id))
}

pub fn read_escrow(e: &Env, escrow_id: u64) -> Escrow {
    e.storage()
        .persistent()
        .get(&DataKey::Escrow(escrow_id))
        .unwrap()
}

pub fn write_escrow(e: &Env, escrow_id: u64, escrow: &Escrow) {
    let key = DataKey::Escrow(escrow_id);
    e.storage().persistent().set(&key, escrow);
    registry_core::extend_persistent_ttl(e, &key);
}
