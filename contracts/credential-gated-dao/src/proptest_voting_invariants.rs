//! Property-based tests for credential-gated-dao voting invariants.
//!
//! Covers invariants 1–4 from the issue:
//! 1. One vote per voter per proposal.
//! 2. The tally is exact.
//! 3. Finalization is a single transition.
//! 4. Cast votes are frozen when schema weights change.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN, Env, Symbol,
};

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
    CredentialGatedDaoContract, CredentialGatedDaoContractClient, ProposalState, MIN_VOTING_PERIOD,
};

struct PropEnv {
    e: Env,
    _admin: Address,
    dao: CredentialGatedDaoContractClient<'static>,
    issuer: Address,
    schema_id: BytesN<32>,
}

fn setup_prop(weight: u32, quorum: u64) -> PropEnv {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = 1_700_000_000);

    let admin = Address::generate(&e);
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

    issuer_client.add_issuer(&issuer, &None, &None, &None);
    let schema_id = schema_client.register_schema(
        &issuer,
        &Symbol::new(&e, "PropSchema"),
        &Symbol::new(&e, "v1"),
        &Bytes::from_slice(&e, b"{\"type\":\"object\"}"),
    );
    dao.set_schema_weight(&schema_id, &weight);

    PropEnv {
        e,
        _admin: admin,
        dao,
        issuer,
        schema_id,
    }
}

fn cred(e: &Env, n: u8) -> Bytes {
    Bytes::from_slice(e, &[b'c', n])
}

proptest! {
    #[test]
    fn prop_one_vote_per_voter(
        weight in 1u32..=100u32,
    ) {
        let ctx = setup_prop(weight, 1);
        let proposer = Address::generate(&ctx.e);
        let voter = Address::generate(&ctx.e);
        let proposal_id = ctx.dao.create_proposal(
            &proposer,
            &Bytes::from_slice(&ctx.e, b"meta"),
            &MIN_VOTING_PERIOD,
        );

        ctx.dao.vote(
            &proposal_id,
            &voter,
            &ctx.issuer,
            &cred(&ctx.e, 1),
            &ctx.schema_id,
            &true,
        );

        let second = ctx.dao.try_vote(
            &proposal_id,
            &voter,
            &ctx.issuer,
            &cred(&ctx.e, 2),
            &ctx.schema_id,
            &false,
        );
        prop_assert!(second.is_err());
    }

    #[test]
    fn prop_tally_exact(
        weight in 1u32..=50u32,
        yes_votes in 0u32..=5u32,
        no_votes in 0u32..=5u32,
    ) {
        let total_voters = yes_votes + no_votes;
        let quorum = (weight as u64) * (total_voters as u64);
        let ctx = setup_prop(weight, quorum.max(1));
        let proposer = Address::generate(&ctx.e);
        let proposal_id = ctx.dao.create_proposal(
            &proposer,
            &Bytes::from_slice(&ctx.e, b"meta"),
            &MIN_VOTING_PERIOD,
        );

        let mut expected_yes = 0u64;
        let mut expected_no = 0u64;

        for i in 0..yes_votes {
            let voter = Address::generate(&ctx.e);
            ctx.dao.vote(
                &proposal_id,
                &voter,
                &ctx.issuer,
                &cred(&ctx.e, i as u8),
                &ctx.schema_id,
                &true,
            );
            expected_yes += weight as u64;
        }

        for i in 0..no_votes {
            let voter = Address::generate(&ctx.e);
            ctx.dao.vote(
                &proposal_id,
                &voter,
                &ctx.issuer,
                &cred(&ctx.e, 100 + i as u8),
                &ctx.schema_id,
                &false,
            );
            expected_no += weight as u64;
        }

        let proposal = ctx.dao.get_proposal(&proposal_id);
        prop_assert_eq!(proposal.yes_weight, expected_yes);
        prop_assert_eq!(proposal.no_weight, expected_no);
        prop_assert_eq!(proposal.yes_weight + proposal.no_weight, expected_yes + expected_no);
    }

    #[test]
    fn prop_finalize_single_transition(
        weight in 1u32..=20u32,
    ) {
        let ctx = setup_prop(weight, weight as u64);
        let proposer = Address::generate(&ctx.e);
        let voter = Address::generate(&ctx.e);
        let proposal_id = ctx.dao.create_proposal(
            &proposer,
            &Bytes::from_slice(&ctx.e, b"meta"),
            &MIN_VOTING_PERIOD,
        );
        ctx.dao.vote(
            &proposal_id,
            &voter,
            &ctx.issuer,
            &cred(&ctx.e, 1),
            &ctx.schema_id,
            &true,
        );

        let proposal = ctx.dao.get_proposal(&proposal_id);
        ctx.e.ledger().with_mut(|l| l.timestamp = proposal.closes_at);
        ctx.dao.finalize(&proposal_id);
        let after = ctx.dao.get_proposal(&proposal_id);
        prop_assert_ne!(after.state, ProposalState::Open);

        let again = ctx.dao.try_finalize(&proposal_id);
        prop_assert!(again.is_err());
    }

    #[test]
    fn prop_weight_frozen_at_cast_time(
        initial in 1u32..=20u32,
        updated in 21u32..=40u32,
    ) {
        let ctx = setup_prop(initial, initial as u64);
        let proposer = Address::generate(&ctx.e);
        let voter = Address::generate(&ctx.e);
        let proposal_id = ctx.dao.create_proposal(
            &proposer,
            &Bytes::from_slice(&ctx.e, b"meta"),
            &MIN_VOTING_PERIOD,
        );

        ctx.dao.vote(
            &proposal_id,
            &voter,
            &ctx.issuer,
            &cred(&ctx.e, 1),
            &ctx.schema_id,
            &true,
        );
        ctx.dao.set_schema_weight(&ctx.schema_id, &updated);

        let proposal = ctx.dao.get_proposal(&proposal_id);
        prop_assert_eq!(proposal.yes_weight, initial as u64);
    }
}
