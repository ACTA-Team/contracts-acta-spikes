//! Shared registry-core crate for VC registries.
//!
//! Provides the common building blocks that the three registries
//! (vc-issuer-registry, vc-schema-registry, vc-revocation-registry)
//! converged on independently:
//!
//! - TTL constants and helpers (`extend_instance_ttl`, `extend_persistent_ttl`)
//! - Admin storage helpers (`has_admin`, `read_admin`, `write_admin`)
//! - Auth guard helpers (`require_admin`, `require_admin_or`)
//! - Shared error codes with a documented numbering convention
//! - Input-size validation helper (`validate_max_bytes`)
//!
//! # Error-code numbering convention
//!
//! Error codes **1–9** are reserved for shared `CommonError` variants.
//! Contract-specific error codes MUST start at **10** (or any value > 9)
//! and MUST NOT collide with codes used by other contracts.
//!
//! | Code | Variant            | Contracts                  |
//! |------|--------------------|----------------------------|
//! | 1    | AlreadyInitialized | All three                  |
//! | 2    | NotFound           | All three (domain-scoped)  |
//! | 3    | AlreadyExists      | All three (domain-scoped)  |
//! | 4    | NotInitialized     | All three                  |
//! | 5    | Unauthorized       | vc-schema-registry + all   |
//! | 6    | InvalidInput       | All three                  |
//! | 7–9  | Reserved           | Future shared use          |

#![no_std]

use soroban_sdk::{contracterror, contracttype, panic_with_error, Address, Bytes, Env};

// ---------------------------------------------------------------------------
// TTL constants (~5 s ledger close): 518_400 ≈ 30 days, 3_110_400 ≈ 180 days.
// ---------------------------------------------------------------------------

/// TTL threshold for instance storage (30 days).
pub const INSTANCE_TTL_THRESHOLD: u32 = 518_400;

/// TTL extension target for instance storage (180 days).
pub const INSTANCE_TTL_EXTEND_TO: u32 = 3_110_400;

/// TTL threshold for persistent storage (30 days).
pub const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;

/// TTL extension target for persistent storage (180 days).
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 3_110_400;

/// Default maximum byte length for metadata / credential ID fields.
pub const DEFAULT_MAX_BYTES: u32 = 256;

// ---------------------------------------------------------------------------
// Shared error codes (Error(Contract, code))
// ---------------------------------------------------------------------------

/// Common error codes shared across all three registries.
///
/// Contract-specific codes start at **10** to avoid collisions with the
/// shared 1–9 range.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum CommonError {
    /// `initialize()` has already been called.
    AlreadyInitialized = 1,
    /// Domain-scoped "not found" — the exact entity depends on the caller.
    NotFound = 2,
    /// Domain-scoped "already exists" — the exact entity depends on the caller.
    AlreadyExists = 3,
    /// Contract has not been initialized yet.
    NotInitialized = 4,
    /// Caller is not authorized for this operation.
    Unauthorized = 5,
    /// Input field exceeds the maximum allowed size.
    InvalidInput = 6,
}

// ---------------------------------------------------------------------------
// Admin storage helpers
// ---------------------------------------------------------------------------

/// Storage key enum used by the shared admin helpers.
/// Contracts with additional storage keys should define their own `DataKey`
/// enum and **not** re-export `registry_core::DataKey`.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Global admin (singleton, instance storage).
    Admin,
}

/// Returns `true` if the admin address has been written to instance storage.
pub fn has_admin(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Admin)
}

/// Reads the stored admin address. Panics at the host level if not present.
pub fn read_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

/// Writes the admin address to instance storage.
pub fn write_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

// ---------------------------------------------------------------------------
// Auth guard helpers
// ---------------------------------------------------------------------------

/// Panics with `CommonError::NotInitialized` if no admin is stored, or with a
/// host auth error if the caller is not the stored admin.
pub fn require_admin(e: &Env) {
    if !has_admin(e) {
        panic_with_error!(e, CommonError::NotInitialized);
    }
    let admin = read_admin(e);
    admin.require_auth();
}

/// Panics with `CommonError::NotInitialized` if no admin is stored, or with
/// `CommonError::Unauthorized` if the caller is neither the admin nor the
/// provided `other` address. The host auth check (`require_auth`) is performed
/// on whichever of `caller` or `other` passed the identity check.
///
/// This is used by vc-schema-registry where the author-or-admin model
/// requires a different auth guard than admin-only.
pub fn require_admin_or(e: &Env, caller: &Address, other: &Address) {
    if !has_admin(e) {
        panic_with_error!(e, CommonError::NotInitialized);
    }
    let admin = read_admin(e);
    if caller == &admin || caller == other {
        caller.require_auth();
        return;
    }
    panic_with_error!(e, CommonError::Unauthorized);
}

// ---------------------------------------------------------------------------
// TTL helpers
// ---------------------------------------------------------------------------

/// Extend the instance storage TTL for the contract.
pub fn extend_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

/// Extend the persistent storage TTL for a specific key.
///
/// Calls `extend_ttl(threshold, extend_to)` on the persistent storage entry
/// identified by `key`. Contracts MUST call this after writing any persistent
/// entry that should not expire.
pub fn extend_persistent_ttl(e: &Env, key: &impl soroban_sdk::IntoVal<Env, soroban_sdk::Val>) {
    e.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

// ---------------------------------------------------------------------------
// Input validation helpers
// ---------------------------------------------------------------------------

/// Validates that `field` does not exceed `max_bytes`. Panics with
/// `CommonError::InvalidInput` if it does.
pub fn validate_max_bytes(e: &Env, field: &Bytes, max_bytes: u32) {
    if field.len() > max_bytes {
        panic_with_error!(e, CommonError::InvalidInput);
    }
}
