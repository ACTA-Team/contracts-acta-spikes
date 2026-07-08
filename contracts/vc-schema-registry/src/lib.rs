//! VC Schema Registry Contract
//!
//! Soroban contract for on-chain registration and governance of verifiable
//! credential schemas. Authors register schemas by id with a URI pointing to
//! the schema document; admins or the original author may deprecate a schema.

#![no_std]

pub mod contract;
pub mod error;
pub mod events;
pub mod storage;

#[cfg(test)]
mod test;
