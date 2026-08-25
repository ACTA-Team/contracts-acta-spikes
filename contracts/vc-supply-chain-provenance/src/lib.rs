//! VC Supply Chain Provenance Contract
//!
//! Tracks custody as an append-only chain, ends in a terminal sealed state,
//! and rejects batches whose origin certificate has been revoked.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;

#[cfg(test)]
mod proptest_custody_invariants;
