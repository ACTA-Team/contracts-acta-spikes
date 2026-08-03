//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.
//!
//! Error codes 1–9 are shared across all registries (see `registry_core::CommonError`):
//!   1 = AlreadyInitialized
//!   4 = NotInitialized
//!   6 = Unauthorized
//! Contract-specific codes start at 10; this contract uses:
//!   2 = SchemaNotFound
//!   3 = SchemaAlreadyExists
//!   5 = AlreadyDeprecated
//!
//! Note: codes 2, 3, and 5 predate the shared-error convention and are kept
//! for backward compatibility. Future contracts should use codes ≥ 10.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// `initialize` has already been called.
    AlreadyInitialized = 1,
    /// Schema ID not found in the registry.
    SchemaNotFound = 2,
    /// A schema with the same `(author, name, version)` triple already exists.
    SchemaAlreadyExists = 3,
    /// Contract has not been initialized yet.
    NotInitialized = 4,
    /// Schema is already deprecated; cannot deprecate again.
    AlreadyDeprecated = 5,
    /// Caller is neither the contract admin nor the schema author.
    Unauthorized = 6,
}
