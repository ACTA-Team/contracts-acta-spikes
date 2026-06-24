//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.

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
