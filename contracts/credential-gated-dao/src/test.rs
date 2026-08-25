extern crate std;

use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN, Env, Symbol,
};

use registry_core::CommonError;
use vc_issuer_registry_contract::contract::{
    VcIssuerRegistryContract, VcIssuerRegistryContractClient,
};
use vc_revocation_registry_contract::contract::{
    VcRevocationRegistryContract, VcRevocationRegistryContractClient,
};
use vc_schema_registry_contract::contract::{
    VcSchemaRegistryContract, VcSchemaRegistryContractClient,
};
use vc_verifier_contract::contract::{VcVerifierContract, VcVerifierContractClient};

use crate::contract::{
    CredentialGatedDaoContract, CredentialGatedDaoContractClient, ProposalState, MAX_VOTING_PERIOD,
    MIN_VOTING_PERIOD,
};
use crate::error::ContractError;

struct TestEnv<'a> {
    e: Env,
    admin: Address,
    proposer: Address,
    voter: Address,
    issuer: Address,
    dao: CredentialGatedDaoContractClient<'a>,
    _verifier: VcVerifierContractClient<'a>,
    verifier_id: soroban_sdk::Address,
    issuer_client: VcIssuerRegistryContractClient<'a>,
    schema_client: VcSchemaRegistryContractClient<'a>,
    revocation_client: VcRevocationRegistryContractClient<'a>,
}

fn setup(quorum: u64) -> TestEnv<'static> {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = 1_700_000_000);

    let admin = Address::generate(&e);
    let proposer = Address::generate(&e);
    let voter = Address::generate(&e);
    let issuer = Address::generate(&e);

    let issuer_reg_id = e.register(VcIssuerRegistryContract, ());
    let schema_reg_id = e.register(VcSchemaRegistryContract, ());
    let revocation_reg_id = e.register(VcRevocationRegistryContract, ());
    let verifier_id = e.register(VcVerifierContract, ());
    let dao_id = e.register(CredentialGatedDaoContract, ());

    let issuer_client = VcIssuerRegistryContractClient::new(&e, &issuer_reg_id);
    let schema_client = VcSchemaRegistryContractClient::new(&e, &schema_reg_id);
    let revocation_client = VcRevocationRegistryContractClient::new(&e, &revocation_reg_id);
    let verifier = VcVerifierContractClient::new(&e, &verifier_id);
    let dao = CredentialGatedDaoContractClient::new(&e, &dao_id);

    issuer_client.initialize(&admin);
    schema_client.initialize(&admin);
    revocation_client.initialize(&admin);
    verifier.initialize(&admin, &issuer_reg_id, &schema_reg_id, &revocation_reg_id);
    dao.initialize(&admin, &verifier_id, &quorum);

    TestEnv {
        e,
        admin,
        proposer,
        voter,
        issuer,
        dao,
        _verifier: verifier,
        verifier_id,
        issuer_client,
        schema_client,
        revocation_client,
    }
}

fn metadata(e: &Env) -> Bytes {
    Bytes::from_slice(e, b"proposal metadata")
}

fn credential_id(e: &Env, suffix: &str) -> Bytes {
    Bytes::from_slice(e, suffix.as_bytes())
}

fn register_issuer_and_schema(ctx: &TestEnv) -> BytesN<32> {
    ctx.issuer_client
        .add_issuer(&ctx.issuer, &None, &None, &None);
    ctx.schema_client.register_schema(
        &ctx.issuer,
        &Symbol::new(&ctx.e, "TestSchema"),
        &Symbol::new(&ctx.e, "v1"),
        &Bytes::from_slice(&ctx.e, b"{\"type\":\"object\"}"),
    )
}

fn create_open_proposal(ctx: &TestEnv) -> u64 {
    ctx.dao
        .create_proposal(&ctx.proposer, &metadata(&ctx.e), &MIN_VOTING_PERIOD)
}

#[test]
fn test_create_proposal_sets_voting_window() {
    let ctx = setup(1);
    let before = ctx.e.ledger().timestamp();
    let proposal_id = ctx
        .dao
        .create_proposal(&ctx.proposer, &metadata(&ctx.e), &MIN_VOTING_PERIOD);
    let proposal = ctx.dao.get_proposal(&proposal_id);
    assert_eq!(proposal.created_at, before);
    assert_eq!(proposal.closes_at, before + MIN_VOTING_PERIOD);
    assert_eq!(proposal.state, ProposalState::Open);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_create_proposal_rejects_out_of_range_period() {
    let ctx = setup(1);
    ctx.dao
        .create_proposal(&ctx.proposer, &metadata(&ctx.e), &(MIN_VOTING_PERIOD - 1));
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_vote_requires_weighted_schema() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_vote_with_revoked_credential_fails() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let cred = credential_id(&ctx.e, "cred-revoked");
    ctx.revocation_client.revoke(&ctx.issuer, &cred);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &cred,
        &schema_id,
        &true,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_vote_with_wrong_issuer_fails() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let proposal_id = create_open_proposal(&ctx);
    let wrong_issuer = Address::generate(&ctx.e);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &wrong_issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_vote_twice_fails() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let proposal_id = create_open_proposal(&ctx);
    let cred = credential_id(&ctx.e, "cred-1");
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &cred,
        &schema_id,
        &true,
    );
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &cred,
        &schema_id,
        &true,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_vote_after_close_fails() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let proposal_id = create_open_proposal(&ctx);
    let proposal = ctx.dao.get_proposal(&proposal_id);
    ctx.e
        .ledger()
        .with_mut(|l| l.timestamp = proposal.closes_at);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
}

#[test]
fn test_vote_applies_schema_weight() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &25);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
    let proposal = ctx.dao.get_proposal(&proposal_id);
    assert_eq!(proposal.yes_weight, 25);
    assert_eq!(proposal.no_weight, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_finalize_before_close_fails() {
    let ctx = setup(1);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.finalize(&proposal_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_finalize_twice_fails() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
    let proposal = ctx.dao.get_proposal(&proposal_id);
    ctx.e
        .ledger()
        .with_mut(|l| l.timestamp = proposal.closes_at);
    ctx.dao.finalize(&proposal_id);
    ctx.dao.finalize(&proposal_id);
}

#[test]
fn test_finalize_passes_with_quorum_and_majority() {
    let ctx = setup(10);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
    let proposal = ctx.dao.get_proposal(&proposal_id);
    ctx.e
        .ledger()
        .with_mut(|l| l.timestamp = proposal.closes_at);
    ctx.dao.finalize(&proposal_id);
    let finalized = ctx.dao.get_proposal(&proposal_id);
    assert_eq!(finalized.state, ProposalState::Passed);
}

#[test]
fn test_finalize_rejects_below_quorum() {
    let ctx = setup(100);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
    let proposal = ctx.dao.get_proposal(&proposal_id);
    ctx.e
        .ledger()
        .with_mut(|l| l.timestamp = proposal.closes_at);
    ctx.dao.finalize(&proposal_id);
    let finalized = ctx.dao.get_proposal(&proposal_id);
    assert_eq!(finalized.state, ProposalState::Rejected);
}

#[test]
fn test_finalize_rejects_on_tie() {
    let ctx = setup(20);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);

    let issuer2 = Address::generate(&ctx.e);
    ctx.issuer_client.add_issuer(&issuer2, &None, &None, &None);
    let schema_id2 = ctx.schema_client.register_schema(
        &issuer2,
        &Symbol::new(&ctx.e, "Schema2"),
        &Symbol::new(&ctx.e, "v1"),
        &Bytes::from_slice(&ctx.e, b"{\"type\":\"object\"}"),
    );
    ctx.dao.set_schema_weight(&schema_id2, &10);

    let proposal_id = create_open_proposal(&ctx);
    let voter2 = Address::generate(&ctx.e);

    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-yes"),
        &schema_id,
        &true,
    );
    ctx.dao.vote(
        &proposal_id,
        &voter2,
        &issuer2,
        &credential_id(&ctx.e, "cred-no"),
        &schema_id2,
        &false,
    );

    let proposal = ctx.dao.get_proposal(&proposal_id);
    ctx.e
        .ledger()
        .with_mut(|l| l.timestamp = proposal.closes_at);
    ctx.dao.finalize(&proposal_id);
    let finalized = ctx.dao.get_proposal(&proposal_id);
    assert_eq!(finalized.state, ProposalState::Rejected);
    assert_eq!(finalized.yes_weight, 10);
    assert_eq!(finalized.no_weight, 10);
}

#[test]
fn test_schema_weight_change_does_not_affect_cast_votes() {
    let ctx = setup(10);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &10);
    let proposal_id = create_open_proposal(&ctx);
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
    ctx.dao.set_schema_weight(&schema_id, &100);
    let proposal = ctx.dao.get_proposal(&proposal_id);
    assert_eq!(proposal.yes_weight, 10);
}

#[test]
fn test_set_schema_weight_requires_admin() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    let _stranger = Address::generate(&ctx.e);
    ctx.e.mock_auths(&[]);
    let result = ctx.dao.try_set_schema_weight(&schema_id, &5);
    assert!(result.is_err(), "non-admin must not set schema weight");
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice_fails() {
    let ctx = setup(1);
    ctx.dao.initialize(&ctx.admin, &ctx.verifier_id, &1);
}

#[test]
fn test_max_voting_period_accepted() {
    let ctx = setup(1);
    let proposal_id = ctx
        .dao
        .create_proposal(&ctx.proposer, &metadata(&ctx.e), &MAX_VOTING_PERIOD);
    let proposal = ctx.dao.get_proposal(&proposal_id);
    assert_eq!(proposal.closes_at, proposal.created_at + MAX_VOTING_PERIOD);
}

#[test]
fn test_has_voted_tracks_voter() {
    let ctx = setup(1);
    let schema_id = register_issuer_and_schema(&ctx);
    ctx.dao.set_schema_weight(&schema_id, &5);
    let proposal_id = create_open_proposal(&ctx);
    assert!(!ctx.dao.has_voted(&proposal_id, &ctx.voter));
    ctx.dao.vote(
        &proposal_id,
        &ctx.voter,
        &ctx.issuer,
        &credential_id(&ctx.e, "cred-1"),
        &schema_id,
        &true,
    );
    assert!(ctx.dao.has_voted(&proposal_id, &ctx.voter));
}

#[test]
fn test_version_returns_symbol() {
    let ctx = setup(1);
    let version = ctx.dao.version();
    assert_eq!(version, Symbol::new(&ctx.e, "0_1_0"));
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_create_proposal_rejects_oversized_metadata() {
    let ctx = setup(1);
    let oversized = Bytes::from_slice(&ctx.e, &[0u8; 257]);
    ctx.dao
        .create_proposal(&ctx.proposer, &oversized, &MIN_VOTING_PERIOD);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_create_proposal_rejects_period_above_max() {
    let ctx = setup(1);
    ctx.dao
        .create_proposal(&ctx.proposer, &metadata(&ctx.e), &(MAX_VOTING_PERIOD + 1));
}

// Ensure CommonError codes are wired for shared failures.
#[test]
fn test_common_error_codes_available() {
    let _ = CommonError::NotFound as u32;
    let _ = ContractError::VotingClosed as u32;
}
