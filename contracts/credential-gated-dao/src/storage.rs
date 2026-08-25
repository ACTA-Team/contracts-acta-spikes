//! Storage layout and helpers for credential-gated-dao.
//!
//! Admin helpers and TTL constants are re-exported from `registry_core`.

use soroban_sdk::{contracttype, Address, BytesN, Env};

use crate::contract::Proposal;

/// Contract storage keys.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// VC verifier contract address (instance).
    Verifier,
    /// Minimum total weight required for a proposal to pass (instance).
    QuorumWeight,
    /// Next proposal identifier (instance).
    NextProposalId,
    /// Proposal record keyed by id (persistent).
    Proposal(u64),
    /// Voting weight for a credential schema (persistent).
    SchemaWeight(BytesN<32>),
    /// Whether a voter has cast on a proposal (persistent).
    Voted(u64, Address),
}

pub use registry_core::{extend_instance_ttl, has_admin, read_admin, write_admin};

pub fn has_verifier(e: &Env) -> bool {
    e.storage().instance().has(&DataKey::Verifier)
}

pub fn read_verifier(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Verifier).unwrap()
}

pub fn write_verifier(e: &Env, verifier: &Address) {
    e.storage().instance().set(&DataKey::Verifier, verifier);
}

pub fn read_quorum_weight(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&DataKey::QuorumWeight)
        .unwrap_or(0)
}

pub fn write_quorum_weight(e: &Env, quorum_weight: u64) {
    e.storage()
        .instance()
        .set(&DataKey::QuorumWeight, &quorum_weight);
}

pub fn read_next_proposal_id(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&DataKey::NextProposalId)
        .unwrap_or(1)
}

pub fn write_next_proposal_id(e: &Env, next_id: u64) {
    e.storage()
        .instance()
        .set(&DataKey::NextProposalId, &next_id);
}

pub fn has_proposal(e: &Env, proposal_id: u64) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Proposal(proposal_id))
}

pub fn read_proposal(e: &Env, proposal_id: u64) -> Proposal {
    e.storage()
        .persistent()
        .get(&DataKey::Proposal(proposal_id))
        .unwrap()
}

pub fn write_proposal(e: &Env, proposal_id: u64, proposal: &Proposal) {
    let key = DataKey::Proposal(proposal_id);
    e.storage().persistent().set(&key, proposal);
    registry_core::extend_persistent_ttl(e, &key);
}

pub fn read_schema_weight(e: &Env, schema_id: &BytesN<32>) -> u32 {
    e.storage()
        .persistent()
        .get(&DataKey::SchemaWeight(schema_id.clone()))
        .unwrap_or(0)
}

pub fn write_schema_weight(e: &Env, schema_id: &BytesN<32>, weight: u32) {
    let key = DataKey::SchemaWeight(schema_id.clone());
    e.storage().persistent().set(&key, &weight);
    registry_core::extend_persistent_ttl(e, &key);
}

pub fn has_voted(e: &Env, proposal_id: u64, voter: &Address) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Voted(proposal_id, voter.clone()))
}

pub fn write_voted(e: &Env, proposal_id: u64, voter: &Address) {
    let key = DataKey::Voted(proposal_id, voter.clone());
    e.storage().persistent().set(&key, &true);
    registry_core::extend_persistent_ttl(e, &key);
}
