//! VC Schema Registry Contract
//!
//! Soroban contract for on-chain VC schema governance and discovery.
//! Authors register versioned schema definitions; the contract assigns each
//! schema a deterministic ID derived from `sha256(xdr(author) || xdr(name) || xdr(version))`.
//! Deprecation is non-destructive: deprecated schemas remain readable but are
//! flagged so downstream consumers can reject them.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
