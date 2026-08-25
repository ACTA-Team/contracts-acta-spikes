//! Contract-specific error codes. Codes 1–9 are reserved for `registry_core::CommonError`.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// `amount <= 0` at creation.
    InvalidAmount = 10,
    /// `deadline <= now` at creation.
    DeadlineInPast = 11,
    /// State is already `Claimed` or `Refunded`.
    EscrowNotFunded = 12,
    /// `claim` called at or after the deadline.
    DeadlinePassed = 13,
    /// `refund` called before the deadline.
    DeadlineNotReached = 14,
    /// `verify` returned an invalid result.
    CredentialNotValid = 15,
    /// `claim` caller is not the recorded beneficiary.
    NotBeneficiary = 16,
    /// `refund` caller is not the recorded depositor.
    NotDepositor = 17,
    /// `beneficiary == depositor` at creation.
    SelfEscrow = 18,
}
