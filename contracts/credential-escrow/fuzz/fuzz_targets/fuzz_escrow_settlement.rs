//! Fuzz target for escrow settlement: no escrow ever pays out twice.

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, token::StellarAssetClient, token::TokenClient,
    Address, Bytes, BytesN, Env, Symbol,
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

use credential_escrow_contract::contract::{
    CredentialEscrowContract, CredentialEscrowContractClient, EscrowState,
};

const FUTURE_DEADLINE_OFFSET: u64 = 86_400;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let e = Env::default();
    e.mock_all_auths();

    let base_ts = 1_700_000_000u64;
    e.ledger().with_mut(|l| l.timestamp = base_ts);

    let admin = Address::generate(&e);
    let depositor = Address::generate(&e);
    let beneficiary = Address::generate(&e);
    let issuer = Address::generate(&e);

    let issuer_reg_id = e.register(VcIssuerRegistryContract, ());
    let schema_reg_id = e.register(VcSchemaRegistryContract, ());
    let revocation_reg_id = e.register(VcRevocationRegistryContract, ());
    let verifier_id = e.register(VcVerifierContract, ());
    let escrow_contract = e.register(CredentialEscrowContract, ());

    let issuer_client = VcIssuerRegistryContractClient::new(&e, &issuer_reg_id);
    let schema_client = VcSchemaRegistryContractClient::new(&e, &schema_reg_id);
    let revocation_client = VcRevocationRegistryContractClient::new(&e, &revocation_reg_id);
    let verifier = VcVerifierContractClient::new(&e, &verifier_id);
    let escrow = CredentialEscrowContractClient::new(&e, &escrow_contract);

    issuer_client.initialize(&admin);
    schema_client.initialize(&admin);
    revocation_client.initialize(&admin);
    verifier.initialize(&admin, &issuer_reg_id, &schema_reg_id, &revocation_reg_id);
    escrow.initialize(&admin, &verifier_id);

    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    let token_addr = sac.address();
    let token = TokenClient::new(&e, &token_addr);
    let token_admin = StellarAssetClient::new(&e, &token_addr);
    token_admin.mint(&depositor, &10_000_000_000i128);

    issuer_client.add_issuer(&issuer, &None, &None, &None);
    let schema_id = schema_client.register_schema(
        &issuer,
        &Symbol::new(&e, "FuzzSchema"),
        &Symbol::new(&e, "v1"),
        &Bytes::from_slice(&e, b"{\"type\":\"object\"}"),
    );

    let amount = ((data[0] as i128) % 100_000) + 1;
    let deadline = base_ts + FUTURE_DEADLINE_OFFSET;
    let escrow_id = escrow.create_escrow(
        &depositor,
        &beneficiary,
        &token_addr,
        &amount,
        &schema_id,
        &issuer,
        &deadline,
    );

    let beneficiary_start = token.balance(&beneficiary);
    let depositor_start = token.balance(&depositor);
    let mut claim_successes = 0u32;
    let mut refund_successes = 0u32;

    for (idx, byte) in data.iter().enumerate().skip(1) {
        if idx % 4 == 0 {
            let at_deadline = byte % 2 == 0;
            e.ledger().with_mut(|l| {
                l.timestamp = if at_deadline {
                    deadline
                } else {
                    deadline.saturating_sub(1)
                };
            });
        }

        let cred = Bytes::from_slice(&e, &[*byte, idx as u8]);
        if byte % 2 == 0 {
            if escrow
                .try_claim(&escrow_id, &beneficiary, &cred)
                .is_ok()
            {
                claim_successes += 1;
            }
        } else if escrow.try_refund(&escrow_id, &depositor).is_ok() {
            refund_successes += 1;
        }
    }

    let beneficiary_paid = token.balance(&beneficiary) - beneficiary_start;
    let depositor_paid = token.balance(&depositor) - depositor_start;
    let total_paid = beneficiary_paid + depositor_paid;

    assert!(total_paid <= amount);
    assert!(beneficiary_paid <= amount);
    assert!(depositor_paid <= amount);
    assert!(claim_successes <= 1);
    assert!(refund_successes <= 1);
    assert!(claim_successes == 0 || refund_successes == 0);

    let record = escrow.get_escrow(&escrow_id);
    if total_paid == amount {
        assert!(record.state == EscrowState::Claimed || record.state == EscrowState::Refunded);
    }
});
