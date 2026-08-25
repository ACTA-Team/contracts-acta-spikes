//! Contract-specific error codes. Codes 1–9 are reserved for `registry_core::CommonError`.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Caller is not the current custodian.
    NotCustodian = 10,
    /// Mutation attempted on a sealed batch.
    BatchSealed = 11,
    /// `to == from` on transfer.
    SelfTransfer = 12,
    /// `hops >= MAX_HOPS`.
    HopLimitExceeded = 13,
    /// The credential is revoked in the revocation registry.
    CertificateRevoked = 14,
    /// Sealing or validating with no certificate attached.
    NoCertificateAttached = 15,
    /// `limit > MAX_CHAIN_PAGE`.
    LimitTooLarge = 16,
}
