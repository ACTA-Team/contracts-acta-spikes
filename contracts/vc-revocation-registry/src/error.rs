//! Contract error codes.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// initialize() has already been called.
    AlreadyInitialized = 1,
    /// Contract has not been initialized yet.
    NotInitialized = 2,
    /// VC is already in the revocation list.
    AlreadyRevoked = 3,
    /// VC is not present in the revocation list.
    NotRevoked = 4,
    /// Caller is not the original issuer of the VC.
    UnauthorizedIssuer = 5,
}
