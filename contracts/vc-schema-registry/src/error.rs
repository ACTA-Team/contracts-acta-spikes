//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// initialize() has already been called.
    AlreadyInitialized = 1,
    /// Contract has not been initialized yet.
    NotInitialized = 2,
    /// Schema with this id already registered.
    SchemaAlreadyExists = 3,
    /// Schema not present in registry.
    SchemaNotFound = 4,
    /// URI exceeds the 256-byte limit.
    InvalidUri = 5,
    /// Caller is not the schema author.
    UnauthorizedAuthor = 6,
}
