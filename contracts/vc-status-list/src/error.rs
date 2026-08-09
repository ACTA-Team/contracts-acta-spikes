//! Contract error codes. Exposed as `Error(Contract, #code)` by Soroban.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// The status list has already been created for this issuer and list_id.
    ListAlreadyExists = 1,
    /// The status list does not exist for this issuer and list_id.
    ListNotFound = 2,
    /// The provided index is out of range for the specified list.
    IndexOutOfRange = 3,
    /// The list size exceeds the maximum allowed value.
    SizeTooLarge = 4,
    /// The provided size is zero — lists must have at least one bit.
    SizeZero = 5,
}
