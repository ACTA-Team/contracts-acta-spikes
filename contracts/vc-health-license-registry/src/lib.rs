//! VC Health License Registry Contract
//!
//! Tracks professional licenses whose validity changes with time: expiry,
//! temporary suspension, and permanent revocation. Status is derived at read
//! time from the stored record plus the current ledger timestamp.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;

#[cfg(test)]
mod proptest_license_invariants;
