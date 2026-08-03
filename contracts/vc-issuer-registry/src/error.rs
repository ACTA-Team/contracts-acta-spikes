//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.
//!
//! Error codes 1–9 are shared across all registries (see `registry_core::CommonError`):
//!   1 = AlreadyInitialized
//!   4 = NotInitialized
//! Contract-specific codes start at 10; this contract uses:
//!   2 = IssuerNotFound
//!   3 = IssuerAlreadyExists
//!   5 = InvalidMetadata
//!
//! Note: codes 2, 3, and 5 predate the shared-error convention and are kept
//! for backward compatibility. Future contracts should use codes ≥ 10.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// initialize() has already been called.
    AlreadyInitialized = 1,
    /// Issuer address not found in the registry.
    IssuerNotFound = 2,
    /// Issuer address already registered.
    IssuerAlreadyExists = 3,
    /// Contract has not been initialized yet.
    NotInitialized = 4,
    /// Metadata field exceeds maximum allowed size.
    InvalidMetadata = 5,
}
