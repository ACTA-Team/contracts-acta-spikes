//! Contract entry points for vc-schema-registry.

use crate::error::ContractError;
use crate::events;
use crate::storage::{self, SchemaRecord};
use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env, Symbol,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

contractmeta!(
    key = "Description",
    val = "VC Schema Registry: on-chain schema registry for verifiable credentials",
);

#[contract]
pub struct VcSchemaRegistryContract;

#[contractimpl]
impl VcSchemaRegistryContract {

    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// One-time initializer. Stores the admin address and emits `Initialized`.
    /// Panics with `AlreadyInitialized` if called more than once.
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
    // Schema management
    // -----------------------------------------------------------------------

    /// Register a new schema. `author` must authorize this call.
    ///
    /// The schema ID is derived deterministically as
    /// `sha256(xdr(author) || xdr(name) || xdr(version))` and returned to the
    /// caller. This ID is stable: the same `(author, name, version)` triple
    /// always produces the same ID, on-chain and off-chain alike.
    ///
    /// Panics with `NotInitialized` if the contract has not been initialized.
    /// Panics with `SchemaAlreadyExists` if the triple is already registered.
    pub fn register_schema(
        e: Env,
        author: Address,
        name: Symbol,
        version: Symbol,
        definition: Bytes,
    ) -> BytesN<32> {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        author.require_auth();

        let schema_id = compute_schema_id(&e, &author, &name, &version);

        if storage::has_schema(&e, &schema_id) {
            panic_with_error!(&e, ContractError::SchemaAlreadyExists);
        }

        let record = SchemaRecord {
            author: author.clone(),
            name,
            version,
            definition,
            deprecated: false,
        };
        storage::write_schema(&e, &schema_id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_registered(&e, &schema_id, &author);

        schema_id
    }

    /// Deprecate a registered schema. `caller` must be either the contract
    /// admin or the author of that specific schema.
    ///
    /// Deprecation is **non-destructive**: the `SchemaRecord` remains on-chain
    /// with `deprecated = true`. Downstream consumers should treat deprecated
    /// schemas as invalid for new credential issuance while still being able
    /// to verify credentials that reference them.
    ///
    /// Panics with `NotInitialized` if the contract has not been initialized.
    /// Panics with `SchemaNotFound` if `schema_id` does not exist.
    /// Panics with `Unauthorized` if `caller` is neither admin nor the schema author.
    /// Panics with `AlreadyDeprecated` if the schema is already deprecated.
    pub fn deprecate_schema(e: Env, schema_id: BytesN<32>, caller: Address) {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        caller.require_auth();

        let mut record = storage::read_schema(&e, &schema_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound));

        let admin = storage::read_admin(&e);
        if caller != admin && caller != record.author {
            panic_with_error!(&e, ContractError::Unauthorized);
        }

        if record.deprecated {
            panic_with_error!(&e, ContractError::AlreadyDeprecated);
        }

        record.deprecated = true;
        storage::write_schema(&e, &schema_id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_deprecated(&e, &schema_id);
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    /// Returns the full `SchemaRecord` for the given ID.
    /// Panics with `SchemaNotFound` if the ID is not in the registry.
    pub fn get_schema(e: Env, schema_id: BytesN<32>) -> SchemaRecord {
        storage::extend_instance_ttl(&e);
        storage::read_schema(&e, &schema_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound))
    }

    /// Returns `true` if a schema with the given ID exists in the registry
    /// (regardless of its `deprecated` flag).
    pub fn schema_exists(e: Env, schema_id: BytesN<32>) -> bool {
        storage::extend_instance_ttl(&e);
        storage::has_schema(&e, &schema_id)
    }

    /// Computes and returns the schema ID for a given `(author, name, version)`
    /// triple without writing to storage. Useful for off-chain pre-computation
    /// or UI tools that need to predict the ID before submitting a transaction.
    pub fn schema_id(e: Env, author: Address, name: Symbol, version: Symbol) -> BytesN<32> {
        compute_schema_id(&e, &author, &name, &version)
    }

    /// Returns the current admin address.
    /// Panics with `NotInitialized` if the contract has not been initialized.
    pub fn admin(e: Env) -> Address {
        if !storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::NotInitialized);
        }
        storage::extend_instance_ttl(&e);
        storage::read_admin(&e)
    }

    /// Returns the contract version string (taken from `Cargo.toml` at compile time).
    pub fn version(e: Env) -> soroban_sdk::String {
        soroban_sdk::String::from_str(&e, VERSION)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Computes `sha256(xdr(author) || xdr(name) || xdr(version))`.
///
/// XDR encoding is used for each component so the preimage is unambiguous and
/// matches what off-chain tooling produces when serializing the same `ScVal`
/// representations of each field.
fn compute_schema_id(e: &Env, author: &Address, name: &Symbol, version: &Symbol) -> BytesN<32> {
    let mut preimage = Bytes::new(e);
    preimage.append(&author.clone().to_xdr(e));
    preimage.append(&name.clone().to_xdr(e));
    preimage.append(&version.clone().to_xdr(e));
    e.crypto().sha256(&preimage).to_bytes()
}
