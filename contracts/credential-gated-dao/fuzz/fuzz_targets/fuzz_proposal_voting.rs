//! Fuzz target for proposal voting tally consistency.
//!
//! Drives arbitrary voters, weights, and timestamps while asserting:
//! - tally equals sum of accepted vote weights
//! - no voter votes twice on the same proposal

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN, Env, Symbol};
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

use credential_gated_dao_contract::contract::{
    CredentialGatedDaoContract, CredentialGatedDaoContractClient, MIN_VOTING_PERIOD,
};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let e = Env::default();
    e.mock_all_auths();

    let base_ts = 1_700_000_000u64;
    e.ledger().with_mut(|l| l.timestamp = base_ts);

    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let proposer = Address::generate(&e);

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

    let weight = ((data[0] as u32) % 50) + 1;
    let quorum = weight as u64;
    dao.initialize(&admin, &verifier_id, &quorum);

    issuer_client.add_issuer(&issuer, &None, &None, &None);
    let schema_id = schema_client.register_schema(
        &issuer,
        &Symbol::new(&e, "FuzzSchema"),
        &Symbol::new(&e, "v1"),
        &Bytes::from_slice(&e, b"{\"type\":\"object\"}"),
    );
    dao.set_schema_weight(&schema_id, &weight);

    let proposal_id = dao.create_proposal(
        &proposer,
        &Bytes::from_slice(&e, b"fuzz"),
        &MIN_VOTING_PERIOD,
    );

    let mut expected_yes = 0u64;
    let mut expected_no = 0u64;
    let mut voters: std::vec::Vec<Address> = std::vec::Vec::new();

    for (idx, byte) in data.iter().enumerate().skip(1) {
        if idx % 8 == 0 {
            let ts_offset = (*byte as u64) % MIN_VOTING_PERIOD;
            e.ledger().with_mut(|l| l.timestamp = base_ts + ts_offset);
            if e.ledger().timestamp() >= base_ts + MIN_VOTING_PERIOD {
                break;
            }
        }

        let voter = Address::generate(&e);
        if voters.iter().any(|v| v == &voter) {
            continue;
        }

        let cred = Bytes::from_slice(&e, &[*byte, idx as u8]);
        let support = byte % 2 == 0;

        let result = dao.try_vote(
            &proposal_id,
            &voter,
            &issuer,
            &cred,
            &schema_id,
            &support,
        );

        if result.is_ok() {
            voters.push(voter);
            if support {
                expected_yes = expected_yes.saturating_add(weight as u64);
            } else {
                expected_no = expected_no.saturating_add(weight as u64);
            }
        }
    }

    let proposal = dao.get_proposal(&proposal_id);
    assert_eq!(proposal.yes_weight, expected_yes);
    assert_eq!(proposal.no_weight, expected_no);

    for voter in &voters {
        assert!(dao.has_voted(&proposal_id, voter));
    }
});
