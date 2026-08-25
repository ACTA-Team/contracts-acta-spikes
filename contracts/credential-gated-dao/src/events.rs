//! Contract events for credential-gated-dao.

use soroban_sdk::{contractevent, Address, BytesN, Env};

use crate::contract::ProposalState;

#[contractevent]
pub struct SchemaWeightSet {
    pub schema_id: BytesN<32>,
    pub weight: u32,
}

#[contractevent]
pub struct ProposalCreated {
    pub proposal_id: u64,
    pub proposer: Address,
    pub closes_at: u64,
}

#[contractevent]
pub struct VoteCast {
    pub proposal_id: u64,
    pub voter: Address,
    pub weight: u32,
    pub support: bool,
}

#[contractevent]
pub struct ProposalFinalized {
    pub proposal_id: u64,
    pub yes_weight: u64,
    pub no_weight: u64,
    pub state: ProposalState,
}

pub fn schema_weight_set(e: &Env, schema_id: &BytesN<32>, weight: u32) {
    SchemaWeightSet {
        schema_id: schema_id.clone(),
        weight,
    }
    .publish(e);
}

pub fn proposal_created(e: &Env, proposal_id: u64, proposer: &Address, closes_at: u64) {
    ProposalCreated {
        proposal_id,
        proposer: proposer.clone(),
        closes_at,
    }
    .publish(e);
}

pub fn vote_cast(e: &Env, proposal_id: u64, voter: &Address, weight: u32, support: bool) {
    VoteCast {
        proposal_id,
        voter: voter.clone(),
        weight,
        support,
    }
    .publish(e);
}

pub fn proposal_finalized(
    e: &Env,
    proposal_id: u64,
    yes_weight: u64,
    no_weight: u64,
    state: ProposalState,
) {
    ProposalFinalized {
        proposal_id,
        yes_weight,
        no_weight,
        state,
    }
    .publish(e);
}
