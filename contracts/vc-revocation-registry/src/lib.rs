//! VC Revocation Registry Contract
//!
//! Soroban contract for on-chain revocation tracking for Verifiable Credentials.
//! Allows authorized issuers to revoke their VCs and verifiers to check revocation status.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
