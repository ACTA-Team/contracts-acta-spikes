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
    /// VC already in revocation list.
    AlreadyRevoked = 3,
    /// VC not present in revocation list.
    NotRevoked = 4,
    /// Caller is not the original issuer.
    UnauthorizedIssuer = 5,
}
