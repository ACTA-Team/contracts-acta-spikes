//! Contract entry points for vc-revocation-registry.

use crate::error::ContractError;
use crate::events;
use crate::storage::{self, RevocationRecord};
use soroban_sdk::{contract, contractimpl, contractmeta, panic_with_error, Address, Bytes, Env};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum allowed byte length for credential identifiers.
const MAX_CREDENTIAL_ID_BYTES: u32 = 256;

contractmeta!(
    key = "Description",
    val = "VC Revocation Registry: on-chain credential revocation status tracking",
);

#[contract]
pub struct VcRevocationRegistryContract;

#[contractimpl]
impl VcRevocationRegistryContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// One-time initializer. Stores the admin address. Panics if already called.
    ///
    /// # Arguments
    /// * `admin` - The address authorized to revoke and unrevoke credentials
    ///
    /// # Errors
    /// * `AlreadyInitialized` - if `initialize` has already been called
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
    // Revocation management (admin-only)
    // -----------------------------------------------------------------------

    /// Revoke a credential. Records the credential as revoked with a timestamp.
    /// Fails if the credential is already revoked.
    ///
    /// # Arguments
    /// * `issuer` - The address of the credential issuer
    /// * `credential_id` - The unique identifier for the credential (max 256 bytes)
    ///
    /// # Errors
    /// * `NotInitialized` - if the contract has not been initialized
    /// * `CredentialAlreadyExists` - if the credential is already revoked
    /// * `InvalidCredentialId` - if credential_id exceeds max byte length
    pub fn revoke(e: Env, issuer: Address, credential_id: Bytes) {
        require_admin(&e);
        validate_credential_id(&e, &credential_id);
        if storage::has_revocation(&e, &issuer, &credential_id) {
            panic_with_error!(&e, ContractError::CredentialAlreadyExists);
        }
        let record = RevocationRecord {
            revoked_at: e.ledger().timestamp(),
        };
        storage::write_revocation(&e, &issuer, &credential_id, &record);
        storage::extend_revocation_ttl(&e);
        events::credential_revoked(&e, &issuer, &credential_id);
    }

    /// Unrevoke a credential by removing it from the revocation registry.
    /// Call this when a credential should no longer be considered revoked.
    /// Fails if the credential is not currently revoked.
    ///
    /// # Arguments
    /// * `issuer` - The address of the credential issuer
    /// * `credential_id` - The unique identifier for the credential
    ///
    /// # Errors
    /// * `NotInitialized` - if the contract has not been initialized
    /// * `CredentialNotFound` - if the credential is not revoked
    pub fn unrevoke(e: Env, issuer: Address, credential_id: Bytes) {
        require_admin(&e);
        if !storage::has_revocation(&e, &issuer, &credential_id) {
            panic_with_error!(&e, ContractError::CredentialNotFound);
        }
        storage::remove_revocation(&e, &issuer, &credential_id);
        storage::extend_revocation_ttl(&e);
        events::credential_unrevoked(&e, &issuer, &credential_id);
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    /// Check if a credential is revoked.
    ///
    /// # Arguments
    /// * `issuer` - The address of the credential issuer
    /// * `credential_id` - The unique identifier for the credential
    ///
    /// # Returns
    /// `true` if the credential is revoked, `false` otherwise
    pub fn is_revoked(e: Env, issuer: Address, credential_id: Bytes) -> bool {
        storage::extend_instance_ttl(&e);
        storage::has_revocation(&e, &issuer, &credential_id)
    }

    /// Get the full revocation record for a credential. Panics if not revoked.
    ///
    /// # Arguments
    /// * `issuer` - The address of the credential issuer
    /// * `credential_id` - The unique identifier for the credential
    ///
    /// # Returns
    /// The `RevocationRecord` containing the revocation timestamp
    ///
    /// # Errors
    /// * `CredentialNotFound` - if the credential is not revoked
    pub fn get_revocation(e: Env, issuer: Address, credential_id: Bytes) -> RevocationRecord {
        storage::extend_instance_ttl(&e);
        storage::read_revocation(&e, &issuer, &credential_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::CredentialNotFound))
    }

    /// Returns the current admin address. Panics if contract is not initialized.
    ///
    /// # Returns
    /// The admin address
    ///
    /// # Errors
    /// * `NotInitialized` - if the contract has not been initialized
    pub fn admin(e: Env) -> Address {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        storage::extend_instance_ttl(&e);
        storage::read_admin(&e)
    }

    /// Returns the contract version string.
    ///
    /// # Returns
    /// The version from Cargo.toml
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

/// Validates credential ID field size. Panics with `InvalidCredentialId` if
/// `credential_id` exceeds [`MAX_CREDENTIAL_ID_BYTES`].
fn validate_credential_id(e: &Env, credential_id: &Bytes) {
    if credential_id.len() > MAX_CREDENTIAL_ID_BYTES {
        panic_with_error!(e, ContractError::InvalidCredentialId);
    }
}
