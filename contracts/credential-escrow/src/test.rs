extern crate std;

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

struct TestEnv<'a> {
    e: Env,
    admin: Address,
    depositor: Address,
    beneficiary: Address,
    issuer: Address,
    escrow: CredentialEscrowContractClient<'a>,
    escrow_id: soroban_sdk::Address,
    token: TokenClient<'a>,
    schema_id: BytesN<32>,
    revocation_client: VcRevocationRegistryContractClient<'a>,
}

const ESCROW_AMOUNT: i128 = 1_000_000;
const FUTURE_DEADLINE_OFFSET: u64 = 86_400;

fn setup() -> TestEnv<'static> {
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
    let escrow_id = e.register(CredentialEscrowContract, ());

    let issuer_client = VcIssuerRegistryContractClient::new(&e, &issuer_reg_id);
    let schema_client = VcSchemaRegistryContractClient::new(&e, &schema_reg_id);
    let revocation_client = VcRevocationRegistryContractClient::new(&e, &revocation_reg_id);
    let verifier = VcVerifierContractClient::new(&e, &verifier_id);
    let escrow = CredentialEscrowContractClient::new(&e, &escrow_id);

    issuer_client.initialize(&admin);
    schema_client.initialize(&admin);
    revocation_client.initialize(&admin);
    verifier.initialize(&admin, &issuer_reg_id, &schema_reg_id, &revocation_reg_id);
    escrow.initialize(&admin, &verifier_id);

    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    let token_addr = sac.address();
    let token = TokenClient::new(&e, &token_addr);
    let token_admin = StellarAssetClient::new(&e, &token_addr);
    token_admin.mint(&depositor, &(ESCROW_AMOUNT * 10));

    issuer_client.add_issuer(&issuer, &None, &None, &None);
    let schema_id = schema_client.register_schema(
        &issuer,
        &Symbol::new(&e, "TestSchema"),
        &Symbol::new(&e, "v1"),
        &Bytes::from_slice(&e, b"{\"type\":\"object\"}"),
    );

    TestEnv {
        e,
        admin,
        depositor,
        beneficiary,
        issuer,
        escrow,
        escrow_id,
        token,
        schema_id,
        revocation_client,
    }
}

fn future_deadline(ctx: &TestEnv) -> u64 {
    ctx.e.ledger().timestamp() + FUTURE_DEADLINE_OFFSET
}

fn credential_id(e: &Env, suffix: &str) -> Bytes {
    Bytes::from_slice(e, suffix.as_bytes())
}

fn create_escrow(ctx: &TestEnv, amount: i128, deadline: u64) -> u64 {
    ctx.escrow.create_escrow(
        &ctx.depositor,
        &ctx.beneficiary,
        &ctx.token.address,
        &amount,
        &ctx.schema_id,
        &ctx.issuer,
        &deadline,
    )
}

#[test]
fn test_create_escrow_moves_funds_into_contract() {
    let ctx = setup();
    let deadline = future_deadline(&ctx);
    let depositor_before = ctx.token.balance(&ctx.depositor);
    let contract_before = ctx.token.balance(&ctx.escrow_id);

    let id = create_escrow(&ctx, ESCROW_AMOUNT, deadline);

    assert_eq!(id, 1);
    assert_eq!(
        ctx.token.balance(&ctx.depositor),
        depositor_before - ESCROW_AMOUNT
    );
    assert_eq!(
        ctx.token.balance(&ctx.escrow_id),
        contract_before + ESCROW_AMOUNT
    );

    let escrow = ctx.escrow.get_escrow(&id);
    assert_eq!(escrow.amount, ESCROW_AMOUNT);
    assert_eq!(escrow.state, EscrowState::Funded);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_create_escrow_rejects_non_positive_amount() {
    let ctx = setup();
    create_escrow(&ctx, 0, future_deadline(&ctx));
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_create_escrow_rejects_past_deadline() {
    let ctx = setup();
    let now = ctx.e.ledger().timestamp();
    create_escrow(&ctx, ESCROW_AMOUNT, now);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_create_escrow_rejects_self_escrow() {
    let ctx = setup();
    ctx.escrow.create_escrow(
        &ctx.depositor,
        &ctx.depositor,
        &ctx.token.address,
        &ESCROW_AMOUNT,
        &ctx.schema_id,
        &ctx.issuer,
        &future_deadline(&ctx),
    );
}

#[test]
fn test_claim_with_valid_credential_pays_beneficiary() {
    let ctx = setup();
    let id = create_escrow(&ctx, ESCROW_AMOUNT, future_deadline(&ctx));
    let beneficiary_before = ctx.token.balance(&ctx.beneficiary);

    ctx.escrow
        .claim(&id, &ctx.beneficiary, &credential_id(&ctx.e, "cred-valid"));

    assert_eq!(
        ctx.token.balance(&ctx.beneficiary),
        beneficiary_before + ESCROW_AMOUNT
    );
    assert_eq!(ctx.token.balance(&ctx.escrow_id), 0);

    let escrow = ctx.escrow.get_escrow(&id);
    assert_eq!(escrow.state, EscrowState::Claimed);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_claim_with_revoked_credential_fails() {
    let ctx = setup();
    let cred = credential_id(&ctx.e, "cred-revoked");
    ctx.revocation_client.revoke(&ctx.issuer, &cred);
    let id = create_escrow(&ctx, ESCROW_AMOUNT, future_deadline(&ctx));
    ctx.escrow.claim(&id, &ctx.beneficiary, &cred);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_claim_with_wrong_issuer_fails() {
    let ctx = setup();
    let wrong_issuer = Address::generate(&ctx.e);
    let id = ctx.escrow.create_escrow(
        &ctx.depositor,
        &ctx.beneficiary,
        &ctx.token.address,
        &ESCROW_AMOUNT,
        &ctx.schema_id,
        &wrong_issuer,
        &future_deadline(&ctx),
    );
    ctx.escrow
        .claim(&id, &ctx.beneficiary, &credential_id(&ctx.e, "cred-1"));
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_claim_after_deadline_fails() {
    let ctx = setup();
    let deadline = future_deadline(&ctx);
    let id = create_escrow(&ctx, ESCROW_AMOUNT, deadline);
    ctx.e.ledger().with_mut(|l| l.timestamp = deadline);
    ctx.escrow
        .claim(&id, &ctx.beneficiary, &credential_id(&ctx.e, "cred-1"));
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_claim_twice_fails() {
    let ctx = setup();
    let id = create_escrow(&ctx, ESCROW_AMOUNT, future_deadline(&ctx));
    let cred = credential_id(&ctx.e, "cred-1");
    ctx.escrow.claim(&id, &ctx.beneficiary, &cred);
    ctx.escrow.claim(&id, &ctx.beneficiary, &cred);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_claim_by_non_beneficiary_fails() {
    let ctx = setup();
    let id = create_escrow(&ctx, ESCROW_AMOUNT, future_deadline(&ctx));
    let stranger = Address::generate(&ctx.e);
    ctx.escrow
        .claim(&id, &stranger, &credential_id(&ctx.e, "cred-1"));
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_refund_before_deadline_fails() {
    let ctx = setup();
    let id = create_escrow(&ctx, ESCROW_AMOUNT, future_deadline(&ctx));
    ctx.escrow.refund(&id, &ctx.depositor);
}

#[test]
fn test_refund_after_deadline_returns_funds() {
    let ctx = setup();
    let deadline = future_deadline(&ctx);
    let id = create_escrow(&ctx, ESCROW_AMOUNT, deadline);
    let depositor_before = ctx.token.balance(&ctx.depositor);

    ctx.e.ledger().with_mut(|l| l.timestamp = deadline);
    ctx.escrow.refund(&id, &ctx.depositor);

    assert_eq!(
        ctx.token.balance(&ctx.depositor),
        depositor_before + ESCROW_AMOUNT
    );
    assert_eq!(ctx.token.balance(&ctx.escrow_id), 0);

    let escrow = ctx.escrow.get_escrow(&id);
    assert_eq!(escrow.state, EscrowState::Refunded);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_refund_after_claim_fails() {
    let ctx = setup();
    let id = create_escrow(&ctx, ESCROW_AMOUNT, future_deadline(&ctx));
    ctx.escrow
        .claim(&id, &ctx.beneficiary, &credential_id(&ctx.e, "cred-1"));
    let deadline = ctx.escrow.get_escrow(&id).deadline;
    ctx.e.ledger().with_mut(|l| l.timestamp = deadline);
    ctx.escrow.refund(&id, &ctx.depositor);
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")]
fn test_refund_by_non_depositor_fails() {
    let ctx = setup();
    let deadline = future_deadline(&ctx);
    let id = create_escrow(&ctx, ESCROW_AMOUNT, deadline);
    ctx.e.ledger().with_mut(|l| l.timestamp = deadline);
    let stranger = Address::generate(&ctx.e);
    ctx.escrow.refund(&id, &stranger);
}

#[test]
fn test_escrow_ids_are_strictly_increasing() {
    let ctx = setup();
    let deadline = future_deadline(&ctx);
    let id1 = create_escrow(&ctx, ESCROW_AMOUNT, deadline);
    let id2 = create_escrow(&ctx, ESCROW_AMOUNT, deadline);
    let id3 = create_escrow(&ctx, ESCROW_AMOUNT, deadline);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);

    let deadline2 = future_deadline(&ctx);
    let id = create_escrow(&ctx, ESCROW_AMOUNT, deadline2);
    ctx.escrow
        .claim(&id, &ctx.beneficiary, &credential_id(&ctx.e, "cred-settle"));
    let id4 = create_escrow(&ctx, ESCROW_AMOUNT, future_deadline(&ctx));
    assert_eq!(id4, 5);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice_fails() {
    let ctx = setup();
    let verifier = Address::generate(&ctx.e);
    ctx.escrow.initialize(&ctx.admin, &verifier);
}
