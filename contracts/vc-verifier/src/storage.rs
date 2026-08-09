//! Storage layout and helpers.
//! All config lives in instance storage — it is global, low-frequency, and
//! should share the same TTL as the contract instance.

use soroban_sdk::{contracttype, Address, Env};

const INSTANCE_TTL_THRESHOLD: u32 = 518_400; // ~30 days
const INSTANCE_TTL_EXTEND_TO: u32 = 3_110_400; // ~180 days

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    IssuerRegistry,
    SchemaRegistry,
    RevocationRegistry,
}

// --- Admin ---

pub fn has_admin(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Admin)
}

pub fn read_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn write_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

// --- Registry addresses ---

pub fn read_issuer_registry(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::IssuerRegistry).unwrap()
}

pub fn write_issuer_registry(e: &Env, addr: &Address) {
    e.storage().instance().set(&DataKey::IssuerRegistry, addr);
}

pub fn read_schema_registry(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::SchemaRegistry).unwrap()
}

pub fn write_schema_registry(e: &Env, addr: &Address) {
    e.storage().instance().set(&DataKey::SchemaRegistry, addr);
}

pub fn read_revocation_registry(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::RevocationRegistry).unwrap()
}

pub fn write_revocation_registry(e: &Env, addr: &Address) {
    e.storage().instance().set(&DataKey::RevocationRegistry, addr);
}

// --- TTL ---

pub fn extend_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}
