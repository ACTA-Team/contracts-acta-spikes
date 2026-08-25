//! Contract-specific error codes. Codes 1–9 are reserved for `registry_core::CommonError`.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// `is_issuer_allowed(authority)` returned false.
    AuthorityNotAllowed = 10,
    /// `expires_at <= now` at issue time.
    ExpiryInPast = 11,
    /// `new_expires_at <= max(now, expires_at)`.
    RenewalNotMonotonic = 12,
    /// Mutation attempted on a revoked license.
    LicenseRevoked = 13,
    /// `lift_suspension` on a license that is not suspended.
    NotSuspended = 14,
    /// `until <= now` at suspend time.
    SuspensionInPast = 15,
}
