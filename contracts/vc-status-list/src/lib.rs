//! VC Status List Contract
//!
//! Soroban contract implementing the W3C StatusList2021 pattern for scalable
//! credential revocation. Each issuer maintains one or more bitmap "status
//! lists"; a credential carries an index into a list, and revocation flips a
//! single bit.
//!
//! The bitmap is stored in fixed-size chunks (4 KB each) so that a single-bit
//! update does not rewrite the entire list. Only the owning issuer may write
//! to their lists; anyone may read.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
