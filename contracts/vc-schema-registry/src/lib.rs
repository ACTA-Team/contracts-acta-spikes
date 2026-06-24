//! VC Schema Registry Contract
//!
//! On-chain registry defining the structure Verifiable Credentials must conform to.
//! Supports versioning so existing VCs remain valid when schemas evolve.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;
