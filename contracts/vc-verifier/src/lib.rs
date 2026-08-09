//! VC Verifier Contract
//!
//! Thin orchestrator that performs a single cross-contract verification call
//! against all three registries (issuer, schema, revocation) and returns a
//! structured breakdown so relying parties get one authoritative answer.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
