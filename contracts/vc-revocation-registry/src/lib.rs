//! VC Revocation Registry Contract
//!
//! Soroban contract for on-chain credential revocation status tracking.
//! Maintains a registry of revoked credentials, keyed by issuer and credential ID.
//! Provides admin-controlled revoke/unrevoke operations and read-only queries
//! for verification of credential status.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
