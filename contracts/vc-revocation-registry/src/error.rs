//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.
//!
//! Error codes 1–9 are shared across all registries (see `registry_core::CommonError`):
//!   1 = AlreadyInitialized
//!   4 = NotInitialized
//! Contract-specific codes start at 10; this contract uses:
//!   2 = CredentialNotFound
//!   3 = CredentialAlreadyExists
//!   5 = InvalidCredentialId
//!
//! Note: codes 2, 3, and 5 predate the shared-error convention and are kept
//! for backward compatibility. Future contracts should use codes ≥ 10.

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
