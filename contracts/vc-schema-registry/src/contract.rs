//! Contract entry points for vc-schema-registry.

use crate::error::ContractError;
use crate::events;
use crate::storage::{self, SchemaRecord};
use soroban_sdk::{contract, contractimpl, contractmeta, panic_with_error, Address, Bytes, Env};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum allowed byte length for the `uri` field.
const MAX_URI_BYTES: u32 = 256;

contractmeta!(
    key = "Description",
    val = "VC Schema Registry: on-chain registration and governance of VC schemas",
);

#[contract]
pub struct VcSchemaRegistryContract;

#[contractimpl]
impl VcSchemaRegistryContract {

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
    // Schema management
    // -----------------------------------------------------------------------

    /// Register a new schema. Caller must be the declared `author`.
    /// Fails if `id` is already registered, or `uri` exceeds [`MAX_URI_BYTES`].
    pub fn register_schema(e: Env, id: Bytes, author: Address, uri: Bytes) {
        author.require_auth();
        if storage::has_schema(&e, &id) {
            panic_with_error!(&e, ContractError::SchemaAlreadyExists);
        }
        validate_uri(&e, &uri);
        let record = SchemaRecord {
            author: author.clone(),
            uri,
            deprecated: false,
        };
        storage::write_schema(&e, &id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_registered(&e, &id, &author);
    }

    /// Mark a schema as deprecated. Callable by the contract admin or the
    /// schema's original author; all other callers fail authorization.
    pub fn deprecate_schema(e: Env, id: Bytes, caller: Address) {
        caller.require_auth();
        let mut record = storage::read_schema(&e, &id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound));
        require_admin_or_author(&e, &caller, &record);
        record.deprecated = true;
        storage::write_schema(&e, &id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_deprecated(&e, &id);
    }

    /// Update the URI for an existing schema. Only the original author may
    /// call this. Fails if `uri` exceeds [`MAX_URI_BYTES`].
    pub fn update_schema_uri(e: Env, id: Bytes, caller: Address, uri: Bytes) {
        caller.require_auth();
        let mut record = storage::read_schema(&e, &id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound));
        if record.author != caller {
            panic_with_error!(&e, ContractError::NotAuthorized);
        }
        validate_uri(&e, &uri);
        record.uri = uri;
        storage::write_schema(&e, &id, &record);
        storage::extend_instance_ttl(&e);
        events::schema_uri_updated(&e, &id);
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    /// Returns the full record for a schema, or panics with SchemaNotFound.
    pub fn get_schema(e: Env, id: Bytes) -> SchemaRecord {
        storage::extend_instance_ttl(&e);
        storage::read_schema(&e, &id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::SchemaNotFound))
    }

    /// Returns true if a schema with `id` is registered.
    pub fn schema_exists(e: Env, id: Bytes) -> bool {
        storage::extend_instance_ttl(&e);
        storage::has_schema(&e, &id)
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

/// Panics with `NotAuthorized` unless `caller` is the stored admin or the
/// schema's author. Tolerates an uninitialized admin (author-only check).
fn require_admin_or_author(e: &Env, caller: &Address, record: &SchemaRecord) {
    let is_author = record.author == *caller;
    let is_admin = storage::has_admin(e) && storage::read_admin(e) == *caller;
    if !is_author && !is_admin {
        panic_with_error!(e, ContractError::NotAuthorized);
    }
}

/// Validates `uri` size. Panics with `InvalidUri` if it exceeds [`MAX_URI_BYTES`].
fn validate_uri(e: &Env, uri: &Bytes) {
    if uri.len() > MAX_URI_BYTES {
        panic_with_error!(e, ContractError::InvalidUri);
    }
}
