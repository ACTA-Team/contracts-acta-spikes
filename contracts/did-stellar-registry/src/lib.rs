//! DID Stellar Registry
//!
//! Soroban contract implementing the `did:stellar` DID method.
//! Controllers register self-sovereign DID documents derived deterministically
//! from their Stellar address. The contract enforces controller-only auth;
//! there is no global admin — this is a deliberate departure from the other
//! registries in this workspace.
//!
//! DID derivation: `sha256(xdr(controller_address))` → 32-byte identifier.
//! Documents are bounded to 1024 bytes. Records are never deleted.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
