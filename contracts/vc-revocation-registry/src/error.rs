//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// `initialize()` has already been called.
    AlreadyInitialized = 1,
    /// Credential not found in the revocation registry.
    CredentialNotFound = 2,
    /// Credential already registered in the revocation registry.
    CredentialAlreadyExists = 3,
    /// Contract has not been initialized yet.
    NotInitialized = 4,
    /// Credential ID exceeds maximum allowed size.
    InvalidCredentialId = 5,
}
