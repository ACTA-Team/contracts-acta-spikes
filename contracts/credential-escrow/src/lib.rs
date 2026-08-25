//! Credential Escrow Contract
//!
//! Soroban contract that custodies SEP-41 tokens and settles exactly once:
//! release to the beneficiary on a valid credential, or refund to the depositor
//! after the deadline.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;

#[cfg(test)]
mod proptest_settlement_invariants;
