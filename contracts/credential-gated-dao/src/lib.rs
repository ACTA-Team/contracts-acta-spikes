//! Credential-Gated DAO Contract
//!
//! Soroban governance contract where voting power comes from verifiable
//! credentials rather than token holdings.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;

#[cfg(test)]
mod proptest_voting_invariants;
