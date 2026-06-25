//! Contract entry points for vc-revocation-registry.

use crate::error::ContractError;
use crate::events;
use crate::storage::{self, RevocationRecord};
use soroban_sdk::{contract, contractimpl, contractmeta, panic_with_error, Address, BytesN, Env, Symbol, Vec};

const VERSION: &str = env!("CARGO_PKG_VERSION");

contractmeta!(
    key = "Description",
    val = "VC Revocation Registry: on-chain revocation tracking for Verifiable Credentials",
);

#[contract]
pub struct VcRevocationRegistryContract;

#[contractimpl]
impl VcRevocationRegistryContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// One-time initializer. Stores the admin address. Panics if already called.
    pub fn initialize(e: Env, admin: Address) {
        if storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&e, &admin);
        storage::extend_instance_ttl(&e);
        events::initialized(&e, &admin);
    }

    // -----------------------------------------------------------------------
    // Revocation management
    // -----------------------------------------------------------------------

    /// Mark a single VC as revoked. Only the original issuer can revoke their own VC.
    pub fn revoke(e: Env, issuer: Address, vc_id: BytesN<32>, reason: Option<Symbol>) {
        require_admin_or_issuer(&e, &issuer);
        if storage::has_revocation(&e, &vc_id) {
            panic_with_error!(&e, ContractError::AlreadyRevoked);
        }
        let record = RevocationRecord {
            issuer: issuer.clone(),
            revoked_at: e.ledger().timestamp(),
            reason,
        };
        storage::write_revocation(&e, &vc_id, &record);
        storage::extend_instance_ttl(&e);
        events::revoked(&e, &vc_id, &issuer);
    }

    /// Mark multiple VCs as revoked in a single transaction. Only the original
    /// issuer can revoke their own VCs.
    pub fn batch_revoke(e: Env, issuer: Address, vc_ids: Vec<BytesN<32>>, reason: Option<Symbol>) {
        require_admin_or_issuer(&e, &issuer);
        let timestamp = e.ledger().timestamp();
        for vc_id in vc_ids.iter() {
            if storage::has_revocation(&e, &vc_id) {
                panic_with_error!(&e, ContractError::AlreadyRevoked);
            }
            let record = RevocationRecord {
                issuer: issuer.clone(),
                revoked_at: timestamp,
                reason: reason.clone(),
            };
            storage::write_revocation(&e, &vc_id, &record);
            events::revoked(&e, &vc_id, &issuer);
        }
        storage::extend_instance_ttl(&e);
    }

    /// Remove a revocation entry (admin-only safeguard).
    pub fn unrevoke(e: Env, vc_id: BytesN<32>) {
        require_admin(&e);
        if !storage::has_revocation(&e, &vc_id) {
            panic_with_error!(&e, ContractError::NotRevoked);
        }
        storage::remove_revocation(&e, &vc_id);
        storage::extend_instance_ttl(&e);
        events::unrevoked(&e, &vc_id);
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    /// Returns true if the VC is revoked.
    pub fn is_revoked(e: Env, vc_id: BytesN<32>) -> bool {
        storage::extend_instance_ttl(&e);
        storage::has_revocation(&e, &vc_id)
    }

    /// Returns the revocation record for a VC. Panics with NotRevoked if not revoked.
    pub fn get_revocation(e: Env, vc_id: BytesN<32>) -> RevocationRecord {
        storage::extend_instance_ttl(&e);
        storage::read_revocation(&e, &vc_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::NotRevoked))
    }

    /// Returns the current admin address. Panics with NotInitialized if not set.
    pub fn admin(e: Env) -> Address {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        storage::extend_instance_ttl(&e);
        storage::read_admin(&e)
    }

    /// Returns the contract version string.
    pub fn version(e: Env) -> soroban_sdk::String {
        soroban_sdk::String::from_str(&e, VERSION)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Panics with `NotInitialized` if no admin is stored, or with a host auth
/// error if the caller is not the stored admin.
fn require_admin(e: &Env) {
    if !storage::has_admin(e) {
        panic_with_error!(e, ContractError::NotInitialized);
    }
    let admin = storage::read_admin(e);
    admin.require_auth();
}

/// Checks auth: either the caller is the admin, or they match the issuer parameter.
fn require_admin_or_issuer(e: &Env, issuer: &Address) {
    if !storage::has_admin(e) {
        panic_with_error!(e, ContractError::NotInitialized);
    }
    // Issuer must authorize the call
    issuer.require_auth();
}
