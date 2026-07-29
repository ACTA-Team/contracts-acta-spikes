//! Contract entry points for vc-verifier.

use crate::error::ContractError;
use crate::events;
use crate::storage;
use soroban_sdk::{
    contract, contractclient, contractimpl, contractmeta, contracttype,
    panic_with_error, Address, Bytes, BytesN, Env, Symbol,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Inline client bindings for cross-contract calls.
// These mirror only the methods the verifier actually calls so we avoid
// dragging in the full registry crate with testutils feature enabled.
// ---------------------------------------------------------------------------

#[contractclient(name = "IssuerRegistryClient")]
pub trait IssuerRegistryTrait {
    fn is_issuer_allowed(env: Env, issuer: Address) -> bool;
}

#[contractclient(name = "SchemaRegistryClient")]
pub trait SchemaRegistryTrait {
    fn schema_exists(env: Env, schema_id: BytesN<32>) -> bool;
    fn get_schema(env: Env, schema_id: BytesN<32>) -> SchemaRecord;
}

#[contractclient(name = "RevocationRegistryClient")]
pub trait RevocationRegistryTrait {
    fn is_revoked(env: Env, issuer: Address, credential_id: Bytes) -> bool;
}

/// Minimal schema record we care about — just the deprecated flag.
#[contracttype]
#[derive(Clone)]
pub struct SchemaRecord {
    pub author: Address,
    pub name: Symbol,
    pub version: Symbol,
    pub definition: Bytes,
    pub deprecated: bool,
}

contractmeta!(
    key = "Description",
    val = "VC Verifier: orchestrator that answers the single verification question for relying parties",
);

/// The verification breakdown returned by `verify`.
///
/// Policy: `valid` is `true` only when the issuer is allowed, the schema
/// exists, it is **not** deprecated, and the credential is **not** revoked.
/// A deprecated schema is treated as fatal — issuers must migrate before
/// new credentials can be considered valid.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationResult {
    /// Overall validity — `true` only when all four conditions pass.
    pub valid: bool,
    /// The issuer is registered and currently on the allowlist.
    pub issuer_allowed: bool,
    /// A schema with the given ID exists in the schema registry.
    pub schema_exists: bool,
    /// The schema has been deprecated.
    pub schema_deprecated: bool,
    /// The credential has been revoked.
    pub revoked: bool,
}

#[contract]
pub struct VcVerifierContract;

#[contractimpl]
impl VcVerifierContract {
    // -----------------------------------------------------------------------
    // Initialization (one-time, admin-only)
    // -----------------------------------------------------------------------

    /// Wire the verifier to the three registries. Admin-only, one time.
    pub fn initialize(
        e: Env,
        admin: Address,
        issuer_registry: Address,
        schema_registry: Address,
        revocation_registry: Address,
    ) {
        if storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&e, &admin);
        storage::write_issuer_registry(&e, &issuer_registry);
        storage::write_schema_registry(&e, &schema_registry);
        storage::write_revocation_registry(&e, &revocation_registry);
        storage::extend_instance_ttl(&e);
        events::initialized(
            &e,
            &admin,
            &issuer_registry,
            &schema_registry,
            &revocation_registry,
        );
    }

    // -----------------------------------------------------------------------
    // Registry address management (admin-only, updatable)
    // -----------------------------------------------------------------------

    /// Update the issuer registry address (e.g. after redeployment).
    pub fn set_issuer_registry(e: Env, new_address: Address) {
        require_admin(&e);
        storage::write_issuer_registry(&e, &new_address);
        storage::extend_instance_ttl(&e);
        events::issuer_registry_updated(&e, &new_address);
    }

    /// Update the schema registry address.
    pub fn set_schema_registry(e: Env, new_address: Address) {
        require_admin(&e);
        storage::write_schema_registry(&e, &new_address);
        storage::extend_instance_ttl(&e);
        events::schema_registry_updated(&e, &new_address);
    }

    /// Update the revocation registry address.
    pub fn set_revocation_registry(e: Env, new_address: Address) {
        require_admin(&e);
        storage::write_revocation_registry(&e, &new_address);
        storage::extend_instance_ttl(&e);
        events::revocation_registry_updated(&e, &new_address);
    }

    // -----------------------------------------------------------------------
    // Core verification — pure read, no storage writes
    // -----------------------------------------------------------------------

    /// The single call a relying party makes.
    ///
    /// Returns a full breakdown so callers can distinguish
    /// "issuer de-listed" from "credential revoked" from "schema deprecated".
    ///
    /// Uses non-panicking predicates so a missing record returns `valid: false`
    /// instead of aborting. This function performs **no storage writes**.
    pub fn verify(
        e: Env,
        issuer: Address,
        schema_id: BytesN<32>,
        credential_id: Bytes,
    ) -> VerificationResult {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }

        let issuer_client =
            IssuerRegistryClient::new(&e, &storage::read_issuer_registry(&e));
        let schema_client =
            SchemaRegistryClient::new(&e, &storage::read_schema_registry(&e));
        let revocation_client =
            RevocationRegistryClient::new(&e, &storage::read_revocation_registry(&e));

        // 1. Issuer check — non-panicking predicate
        let issuer_allowed = issuer_client.is_issuer_allowed(&issuer);

        // 2 & 3. Schema checks — use schema_exists first to avoid panic on missing
        let schema_exists = schema_client.schema_exists(&schema_id);
        let schema_deprecated = if schema_exists {
            schema_client.get_schema(&schema_id).deprecated
        } else {
            false
        };

        // 4. Revocation check — non-panicking predicate
        let revoked = revocation_client.is_revoked(&issuer, &credential_id);

        let valid = issuer_allowed && schema_exists && !schema_deprecated && !revoked;

        VerificationResult {
            valid,
            issuer_allowed,
            schema_exists,
            schema_deprecated,
            revoked,
        }
    }

    // -----------------------------------------------------------------------
    // Read-only accessors
    // -----------------------------------------------------------------------

    pub fn admin(e: Env) -> Address {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        storage::extend_instance_ttl(&e);
        storage::read_admin(&e)
    }

    pub fn issuer_registry(e: Env) -> Address {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        storage::extend_instance_ttl(&e);
        storage::read_issuer_registry(&e)
    }

    pub fn schema_registry(e: Env) -> Address {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        storage::extend_instance_ttl(&e);
        storage::read_schema_registry(&e)
    }

    pub fn revocation_registry(e: Env) -> Address {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        storage::extend_instance_ttl(&e);
        storage::read_revocation_registry(&e)
    }

    pub fn version(e: Env) -> soroban_sdk::String {
        soroban_sdk::String::from_str(&e, VERSION)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn require_admin(e: &Env) {
    if !storage::has_admin(e) {
        panic_with_error!(e, ContractError::NotInitialized);
    }
    let admin = storage::read_admin(e);
    admin.require_auth();
}
