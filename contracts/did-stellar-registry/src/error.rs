//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// A DID for this controller is already registered.
    DidAlreadyExists = 1,
    /// No DID record found for the given DID bytes.
    DidNotFound = 2,
    /// The DID has been deactivated; mutation is not permitted.
    DidDeactivated = 3,
    /// Caller is not the controller of this DID.
    Unauthorized = 4,
    /// Document byte length exceeds MAX_DOCUMENT_BYTES (1024).
    DocumentTooLarge = 5,
}
