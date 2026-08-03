//! Storage layout and helpers.
//! Instance storage  → admin (global config, low-frequency reads).
//! Persistent storage → per-issuer records (long-lived, keyed by Address).
//!
//! Admin helpers and TTL constants are re-exported from `registry_core`.
//! Contract-specific error codes start at 10 (1–9 are shared in `CommonError`).

use registry_core;
use soroban_sdk::{contracttype, Address, Bytes, Env, Symbol};

/// Storage keys separated by role (explicit role isolation).
///
/// Note: `Admin` is provided by `registry_core::DataKey::Admin` and is
/// NOT duplicated here. This enum only adds contract-specific keys.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Issuer registry (per-address persistent storage)
    Issuer(Address),
}

/// On-chain metadata for a registered issuer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerRecord {
    /// Whether this issuer is currently on the allowlist.
    pub allowed: bool,
    /// Human-readable name (optional).
    pub name: Option<Symbol>,
    /// DID document bytes (optional).
    pub did: Option<Bytes>,
    /// URL bytes (optional).
    pub url: Option<Bytes>,
}

// --- Admin (instance) — delegate to registry-core ---

pub use registry_core::{extend_instance_ttl, has_admin, read_admin, write_admin};

// --- Issuer records (persistent) ---

pub fn has_issuer(e: &Env, issuer: &Address) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Issuer(issuer.clone()))
}

pub fn read_issuer(e: &Env, issuer: &Address) -> Option<IssuerRecord> {
    e.storage()
        .persistent()
        .get(&DataKey::Issuer(issuer.clone()))
}

pub fn write_issuer(e: &Env, issuer: &Address, record: &IssuerRecord) {
    let key = DataKey::Issuer(issuer.clone());
    e.storage().persistent().set(&key, record);
    registry_core::extend_persistent_ttl(e, &key);
}

pub fn remove_issuer(e: &Env, issuer: &Address) {
    e.storage()
        .persistent()
        .remove(&DataKey::Issuer(issuer.clone()));
}
