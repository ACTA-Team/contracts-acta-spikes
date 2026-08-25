//! Contract entry points for credential-gated-dao.

use crate::error::ContractError;
use crate::events;
use crate::storage;
use registry_core::{validate_max_bytes, CommonError, DEFAULT_MAX_BYTES};
use soroban_sdk::{
    contract, contractclient, contractimpl, contractmeta, contracttype, panic_with_error, Address,
    Bytes, BytesN, Env, Symbol,
};

const VERSION_SYMBOL: &str = "0_1_0";
pub const MIN_VOTING_PERIOD: u64 = 3_600;
pub const MAX_VOTING_PERIOD: u64 = 2_592_000;

// ---------------------------------------------------------------------------
// Cross-contract binding for vc-verifier (mirrors only fields we read).
// ---------------------------------------------------------------------------

#[contractclient(name = "VerifierClient")]
pub trait VerifierTrait {
    fn verify(
        env: Env,
        issuer: Address,
        schema_id: BytesN<32>,
        credential_id: Bytes,
    ) -> VerificationResult;
}

/// Minimal verification result mirrored from vc-verifier.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationResult {
    pub valid: bool,
    pub issuer_allowed: bool,
    pub schema_exists: bool,
    pub schema_deprecated: bool,
    pub revoked: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalState {
    Open,
    Passed,
    Rejected,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub proposer: Address,
    pub metadata: Bytes,
    pub created_at: u64,
    pub closes_at: u64,
    pub yes_weight: u64,
    pub no_weight: u64,
    pub state: ProposalState,
}

contractmeta!(
    key = "Description",
    val = "Credential-Gated DAO: governance weighted by verifiable credentials",
);

#[contract]
pub struct CredentialGatedDaoContract;

#[contractimpl]
impl CredentialGatedDaoContract {
    /// Idempotent. Second call fails with `CommonError::AlreadyInitialized`.
    pub fn initialize(e: Env, admin: Address, verifier: Address, quorum_weight: u64) {
        if storage::has_admin(&e) {
            panic_with_error!(&e, CommonError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&e, &admin);
        storage::write_verifier(&e, &verifier);
        storage::write_quorum_weight(&e, quorum_weight);
        storage::write_next_proposal_id(&e, 1);
        storage::extend_instance_ttl(&e);
    }

    /// Admin only. `weight == 0` means the schema does not grant voting rights.
    pub fn set_schema_weight(e: Env, schema_id: BytesN<32>, weight: u32) {
        require_admin(&e);
        storage::write_schema_weight(&e, &schema_id, weight);
        storage::extend_instance_ttl(&e);
        events::schema_weight_set(&e, &schema_id, weight);
    }

    pub fn get_schema_weight(e: Env, schema_id: BytesN<32>) -> u32 {
        require_initialized(&e);
        storage::read_schema_weight(&e, &schema_id)
    }

    /// Returns a strictly increasing proposal id.
    pub fn create_proposal(e: Env, proposer: Address, metadata: Bytes, voting_period: u64) -> u64 {
        require_initialized(&e);
        proposer.require_auth();
        validate_max_bytes(&e, &metadata, DEFAULT_MAX_BYTES);
        if !(MIN_VOTING_PERIOD..=MAX_VOTING_PERIOD).contains(&voting_period) {
            panic_with_error!(&e, ContractError::InvalidVotingPeriod);
        }

        let now = e.ledger().timestamp();
        let proposal_id = storage::read_next_proposal_id(&e);
        storage::write_next_proposal_id(&e, proposal_id + 1);

        let proposal = Proposal {
            proposer: proposer.clone(),
            metadata,
            created_at: now,
            closes_at: now + voting_period,
            yes_weight: 0,
            no_weight: 0,
            state: ProposalState::Open,
        };

        storage::write_proposal(&e, proposal_id, &proposal);
        storage::extend_instance_ttl(&e);
        events::proposal_created(&e, proposal_id, &proposer, proposal.closes_at);
        proposal_id
    }

    pub fn vote(
        e: Env,
        proposal_id: u64,
        voter: Address,
        issuer: Address,
        credential_id: Bytes,
        schema_id: BytesN<32>,
        support: bool,
    ) {
        require_initialized(&e);
        voter.require_auth();

        if !storage::has_proposal(&e, proposal_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let mut proposal = storage::read_proposal(&e, proposal_id);
        if proposal.state != ProposalState::Open {
            panic_with_error!(&e, ContractError::AlreadyFinalized);
        }

        let now = e.ledger().timestamp();
        if now >= proposal.closes_at {
            panic_with_error!(&e, ContractError::VotingClosed);
        }

        if storage::has_voted(&e, proposal_id, &voter) {
            panic_with_error!(&e, ContractError::AlreadyVoted);
        }

        let weight = storage::read_schema_weight(&e, &schema_id);
        if weight == 0 {
            panic_with_error!(&e, ContractError::SchemaNotWeighted);
        }

        let verifier_addr = storage::read_verifier(&e);
        let verifier = VerifierClient::new(&e, &verifier_addr);
        let result = verifier.verify(&issuer, &schema_id, &credential_id);
        if !result.valid {
            panic_with_error!(&e, ContractError::CredentialNotValid);
        }

        let weight_u64 = weight as u64;
        if support {
            proposal.yes_weight = proposal
                .yes_weight
                .checked_add(weight_u64)
                .unwrap_or_else(|| panic_with_error!(&e, ContractError::WeightOverflow));
        } else {
            proposal.no_weight = proposal
                .no_weight
                .checked_add(weight_u64)
                .unwrap_or_else(|| panic_with_error!(&e, ContractError::WeightOverflow));
        }

        storage::write_proposal(&e, proposal_id, &proposal);
        storage::write_voted(&e, proposal_id, &voter);
        storage::extend_instance_ttl(&e);
        events::vote_cast(&e, proposal_id, &voter, weight, support);
    }

    /// Permissionless finalization after the voting window closes.
    pub fn finalize(e: Env, proposal_id: u64) {
        require_initialized(&e);

        if !storage::has_proposal(&e, proposal_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let mut proposal = storage::read_proposal(&e, proposal_id);
        if proposal.state != ProposalState::Open {
            panic_with_error!(&e, ContractError::AlreadyFinalized);
        }

        let now = e.ledger().timestamp();
        if now < proposal.closes_at {
            panic_with_error!(&e, ContractError::VotingOpen);
        }

        let quorum = storage::read_quorum_weight(&e);
        let total_weight = proposal.yes_weight.saturating_add(proposal.no_weight);

        proposal.state = if total_weight >= quorum && proposal.yes_weight > proposal.no_weight {
            ProposalState::Passed
        } else {
            ProposalState::Rejected
        };

        storage::write_proposal(&e, proposal_id, &proposal);
        storage::extend_instance_ttl(&e);
        events::proposal_finalized(
            &e,
            proposal_id,
            proposal.yes_weight,
            proposal.no_weight,
            proposal.state.clone(),
        );
    }

    pub fn get_proposal(e: Env, proposal_id: u64) -> Proposal {
        require_initialized(&e);
        if !storage::has_proposal(&e, proposal_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }
        storage::read_proposal(&e, proposal_id)
    }

    pub fn has_voted(e: Env, proposal_id: u64, voter: Address) -> bool {
        require_initialized(&e);
        if !storage::has_proposal(&e, proposal_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }
        storage::has_voted(&e, proposal_id, &voter)
    }

    pub fn version(e: Env) -> Symbol {
        Symbol::new(&e, VERSION_SYMBOL)
    }
}

fn require_initialized(e: &Env) {
    if !storage::has_admin(e) {
        panic_with_error!(e, CommonError::NotInitialized);
    }
}

fn require_admin(e: &Env) {
    if !storage::has_admin(e) {
        panic_with_error!(e, CommonError::NotInitialized);
    }
    let admin = storage::read_admin(e);
    admin.require_auth();
}
