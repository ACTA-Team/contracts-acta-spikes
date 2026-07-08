//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// initialize() has already been called.
    AlreadyInitialized = 1,
    /// Schema id not found in the registry.
    SchemaNotFound = 2,
    /// Schema id already registered.
    SchemaAlreadyExists = 3,
    /// Contract has not been initialized yet.
    NotInitialized = 4,
    /// `uri` exceeds the maximum allowed size.
    InvalidUri = 5,
    /// Caller is neither the admin nor the schema's author.
    NotAuthorized = 6,
}
