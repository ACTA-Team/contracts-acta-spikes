//! Contract entry points for vc-schema-registry.

use crate::error::ContractError;
use crate::events;
use crate::storage::{self, SchemaRecord};
use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, Address, Bytes, BytesN, Env, Symbol,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_URI_BYTES: u32 = 256;

contractmeta!(
    key = "Description",
    val = "VC Schema Registry: on-chain versioned schema registry for Verifiable Credentials",
);

#[contract]
pub struct VcSchemaRegistryContract;

#[contractimpl]
impl VcSchemaRegistryContract {
    /// One-time initializer. Panics if already called.
    pub fn initialize(e: Env, admin: Address) {
        if storage::has_admin(&e) {
            panic_with_error!(&e, ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&e, &admin);
        storage::extend_instance_ttl(&e);
        events::initialized(&e, &admin);
    }

    /// Register a new schema. Panics if schema_id already exists.
    pub fn register_schema(
        e: Env,
        author: Address,
        name: Symbol,
        version: u32,
        schema_id: BytesN<32>,
        content_hash: BytesN<32>,
        uri: Option<Bytes>,
    ) {
        require_initialized(&e);
        author.require_auth();
        validate_uri(&e, &uri);
        if storage::has_schema(&e, &schema_id) {
            panic_with_error!(&e, ContractError::SchemaAlreadyExists);
        }
        let record = SchemaRecord {
            name,
            version,
            author: author.clone(),
            content_hash,
            uri,
            deprecated: false,
        };
        storage::write_schema(&e, &schema_id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_registered(&e, &schema_id, &author);
    }

    /// Mark a schema as deprecated. Callable by admin or the schema author.
    pub fn deprecate_schema(e: Env, caller: Address, schema_id: BytesN<32>) {
        require_initialized(&e);
        let mut record = storage::read_schema(&e, &schema_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound));
        let admin = storage::read_admin(&e);
        if caller != admin && caller != record.author {
            panic_with_error!(&e, ContractError::UnauthorizedAuthor);
        }
        caller.require_auth();
        record.deprecated = true;
        storage::write_schema(&e, &schema_id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_deprecated(&e, &schema_id);
    }

    /// Update the off-chain URI pointer. Callable by the schema author only.
    pub fn update_schema_uri(e: Env, author: Address, schema_id: BytesN<32>, uri: Option<Bytes>) {
        require_initialized(&e);
        author.require_auth();
        validate_uri(&e, &uri);
        let mut record = storage::read_schema(&e, &schema_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound));
        if author != record.author {
            panic_with_error!(&e, ContractError::UnauthorizedAuthor);
        }
        record.uri = uri;
        storage::write_schema(&e, &schema_id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_uri_updated(&e, &schema_id);
    }

    /// Returns the SchemaRecord or panics with SchemaNotFound.
    pub fn get_schema(e: Env, schema_id: BytesN<32>) -> SchemaRecord {
        storage::extend_instance_ttl(&e);
        storage::read_schema(&e, &schema_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound))
    }

    /// Returns true if the schema is registered.
    pub fn schema_exists(e: Env, schema_id: BytesN<32>) -> bool {
        storage::extend_instance_ttl(&e);
        storage::has_schema(&e, &schema_id)
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

fn require_initialized(e: &Env) {
    if !storage::has_admin(e) {
        panic_with_error!(e, ContractError::NotInitialized);
    }
}

fn validate_uri(e: &Env, uri: &Option<Bytes>) {
    if let Some(u) = uri {
        if u.len() > MAX_URI_BYTES {
            panic_with_error!(e, ContractError::InvalidUri);
        }
    }
}
