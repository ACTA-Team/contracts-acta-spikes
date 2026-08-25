//! Storage layout and helpers for vc-health-license-registry.
//!
//! Admin helpers and TTL constants are re-exported from `registry_core`.

use soroban_sdk::{contracttype, Address, BytesN, Env};

use crate::contract::License;

/// Contract storage keys.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Issuer registry contract address (instance).
    IssuerRegistry,
    /// License record keyed by id (persistent).
    License(BytesN<32>),
}

pub use registry_core::{extend_instance_ttl, has_admin, read_admin, write_admin};

pub fn read_issuer_registry(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&DataKey::IssuerRegistry)
        .unwrap()
}

pub fn write_issuer_registry(e: &Env, registry: &Address) {
    e.storage()
        .instance()
        .set(&DataKey::IssuerRegistry, registry);
}

pub fn has_license(e: &Env, license_id: &BytesN<32>) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::License(license_id.clone()))
}

pub fn read_license(e: &Env, license_id: &BytesN<32>) -> License {
    e.storage()
        .persistent()
        .get(&DataKey::License(license_id.clone()))
        .unwrap()
}

pub fn write_license(e: &Env, license_id: &BytesN<32>, license: &License) {
    let key = DataKey::License(license_id.clone());
    e.storage().persistent().set(&key, license);
    registry_core::extend_persistent_ttl(e, &key);
}
