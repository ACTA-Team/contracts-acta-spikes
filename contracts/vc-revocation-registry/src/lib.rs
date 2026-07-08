//! VC Revocation Registry Contract
//!
//! Soroban contract for on-chain VC revocation tracking.
//! Allows authorized issuers to revoke their own VCs and enables
//! verifiers to check revocation status without an off-chain indexer.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
