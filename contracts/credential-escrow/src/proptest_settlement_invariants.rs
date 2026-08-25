//! Property-based tests for credential-escrow settlement invariants.
//!
//! Covers invariants 1–3 from the issue:
//! 1. Each escrow settles at most once.
//! 2. Solvency: contract balance >= sum of Funded escrow amounts.
//! 3. IDs are strictly increasing and never reused.

extern crate std;

use proptest::prelude::*;
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

use crate::contract::{CredentialEscrowContract, CredentialEscrowContractClient, EscrowState};

const FUTURE_DEADLINE_OFFSET: u64 = 86_400;

struct PropEnv {
    e: Env,
    depositor: Address,
    beneficiary: Address,
    issuer: Address,
    escrow: CredentialEscrowContractClient<'static>,
    escrow_contract: Address,
    token: TokenClient<'static>,
    schema_id: BytesN<32>,
}

fn setup_prop(num_escrows: u64, amount_each: i128) -> PropEnv {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = 1_700_000_000);

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
    let total_mint = amount_each
        .saturating_mul(num_escrows as i128)
        .saturating_mul(2);
    token_admin.mint(&depositor, &total_mint);

    issuer_client.add_issuer(&issuer, &None, &None, &None);
    let schema_id = schema_client.register_schema(
        &issuer,
        &Symbol::new(&e, "PropSchema"),
        &Symbol::new(&e, "v1"),
        &Bytes::from_slice(&e, b"{\"type\":\"object\"}"),
    );

    PropEnv {
        e,
        depositor,
        beneficiary,
        issuer,
        escrow,
        escrow_contract,
        token,
        schema_id,
    }
}

fn cred(e: &Env, n: u8) -> Bytes {
    Bytes::from_slice(e, &[b'c', n])
}

fn funded_sum(ctx: &PropEnv, escrow_ids: &[u64]) -> i128 {
    escrow_ids
        .iter()
        .filter_map(|id| {
            let rec = ctx.escrow.get_escrow(id);
            if rec.state == EscrowState::Funded {
                Some(rec.amount)
            } else {
                None
            }
        })
        .sum()
}

proptest! {
    #[test]
    fn prop_settles_at_most_once(
        amount in 1i128..=100_000i128,
        ops in prop::collection::vec(any::<u8>(), 1..=20),
    ) {
        let ctx = setup_prop(1, amount);
        let deadline = ctx.e.ledger().timestamp() + FUTURE_DEADLINE_OFFSET;
        let escrow_id = ctx.escrow.create_escrow(
            &ctx.depositor,
            &ctx.beneficiary,
            &ctx.token.address,
            &amount,
            &ctx.schema_id,
            &ctx.issuer,
            &deadline,
        );

        let beneficiary_start = ctx.token.balance(&ctx.beneficiary);
        let depositor_start = ctx.token.balance(&ctx.depositor);
        let mut paid_beneficiary = 0i128;
        let mut paid_depositor = 0i128;

        for (i, byte) in ops.iter().enumerate() {
            let ts = if i % 3 == 0 {
                deadline.saturating_sub(1)
            } else {
                deadline
            };
            ctx.e.ledger().with_mut(|l| l.timestamp = ts);

            if byte % 2 == 0 {
                if ctx.escrow.try_claim(
                    &escrow_id,
                    &ctx.beneficiary,
                    &cred(&ctx.e, *byte),
                ).is_ok() {
                    paid_beneficiary += amount;
                }
            } else if ctx.escrow.try_refund(
                &escrow_id,
                &ctx.depositor,
            ).is_ok() {
                paid_depositor += amount;
            }
        }

        let total_paid = paid_beneficiary + paid_depositor;
        prop_assert!(total_paid == 0 || total_paid == amount);
        prop_assert!(paid_beneficiary == 0 || paid_depositor == 0);

        prop_assert_eq!(
            ctx.token.balance(&ctx.beneficiary) - beneficiary_start,
            paid_beneficiary
        );
        prop_assert_eq!(
            ctx.token.balance(&ctx.depositor) - depositor_start,
            paid_depositor
        );
    }

    #[test]
    fn prop_solvency(
        num_escrows in 1u64..=5u64,
        amount in 1i128..=50_000i128,
    ) {
        let ctx = setup_prop(num_escrows, amount);
        let mut ids = std::vec::Vec::new();
        let deadline = ctx.e.ledger().timestamp() + FUTURE_DEADLINE_OFFSET;

        for _ in 0..num_escrows {
            let id = ctx.escrow.create_escrow(
                &ctx.depositor,
                &ctx.beneficiary,
                &ctx.token.address,
                &amount,
                &ctx.schema_id,
                &ctx.issuer,
                &deadline,
            );
            ids.push(id);

            let funded = funded_sum(&ctx, &ids);
            prop_assert!(ctx.token.balance(&ctx.escrow_contract) >= funded);
        }
    }

    #[test]
    fn prop_ids_strictly_increasing(
        count in 1u64..=8u64,
        amount in 1i128..=10_000i128,
    ) {
        let ctx = setup_prop(count, amount);
        let base_ts = ctx.e.ledger().timestamp();
        let deadline = base_ts + FUTURE_DEADLINE_OFFSET;
        let mut last_id = 0u64;

        for i in 0..count {
            let id = ctx.escrow.create_escrow(
                &ctx.depositor,
                &ctx.beneficiary,
                &ctx.token.address,
                &amount,
                &ctx.schema_id,
                &ctx.issuer,
                &deadline,
            );
            prop_assert!(id > last_id);
            last_id = id;

            if i % 2 == 0 {
                ctx.escrow.claim(
                    &id,
                    &ctx.beneficiary,
                    &cred(&ctx.e, i as u8),
                );
            } else {
                ctx.e.ledger().with_mut(|l| l.timestamp = deadline);
                ctx.escrow.refund(&id, &ctx.depositor);
                ctx.e.ledger().with_mut(|l| l.timestamp = base_ts);
            }
        }

        let next_id = ctx.escrow.create_escrow(
            &ctx.depositor,
            &ctx.beneficiary,
            &ctx.token.address,
            &amount,
            &ctx.schema_id,
            &ctx.issuer,
            &deadline,
        );
        prop_assert_eq!(next_id, last_id + 1);
    }
}
