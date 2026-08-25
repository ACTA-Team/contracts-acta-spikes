extern crate std;

use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN, Env, Symbol,
};

use vc_revocation_registry_contract::contract::{
    VcRevocationRegistryContract, VcRevocationRegistryContractClient,
};

use crate::contract::{
    BatchState, CustodyHop, VcSupplyChainProvenanceContract, VcSupplyChainProvenanceContractClient,
};

struct TestEnv<'a> {
    e: Env,
    admin: Address,
    certifier: Address,
    provenance: VcSupplyChainProvenanceContractClient<'a>,
    revocation_client: VcRevocationRegistryContractClient<'a>,
}

fn setup() -> TestEnv<'static> {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = 1_700_000_000);

    let admin = Address::generate(&e);
    let certifier = Address::generate(&e);

    let revocation_reg_id = e.register(VcRevocationRegistryContract, ());
    let provenance_id = e.register(VcSupplyChainProvenanceContract, ());

    let revocation_client = VcRevocationRegistryContractClient::new(&e, &revocation_reg_id);
    let provenance = VcSupplyChainProvenanceContractClient::new(&e, &provenance_id);

    revocation_client.initialize(&admin);
    provenance.initialize(&admin, &revocation_reg_id);

    TestEnv {
        e,
        admin,
        certifier,
        provenance,
        revocation_client,
    }
}

fn batch_id(e: &Env, n: u8) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[0] = n;
    BytesN::from_array(e, &bytes)
}

fn metadata(e: &Env, suffix: &str) -> Bytes {
    Bytes::from_slice(e, suffix.as_bytes())
}

fn credential_id(e: &Env, suffix: &str) -> Bytes {
    Bytes::from_slice(e, suffix.as_bytes())
}

fn register_batch(ctx: &TestEnv, id: u8) {
    ctx.provenance.register_batch(
        &ctx.certifier,
        &batch_id(&ctx.e, id),
        &Symbol::new(&ctx.e, "Coffee"),
        &Symbol::new(&ctx.e, "Colombia"),
        &metadata(&ctx.e, "lot-meta"),
    );
}

fn attach_valid_certificate(ctx: &TestEnv, id: u8, cred_suffix: &str) {
    ctx.provenance.attach_certificate(
        &ctx.certifier,
        &batch_id(&ctx.e, id),
        &credential_id(&ctx.e, cred_suffix),
    );
}

#[test]
fn test_register_batch_sets_certifier_as_custodian() {
    let ctx = setup();
    register_batch(&ctx, 1);

    let batch = ctx.provenance.get_batch(&batch_id(&ctx.e, 1));
    assert_eq!(batch.certifier, ctx.certifier);
    assert_eq!(batch.custodian, ctx.certifier);
    assert_eq!(batch.hops, 0);
    assert_eq!(batch.state, BatchState::InTransit);
    assert_eq!(batch.sealed_at, 0);
    assert_eq!(ctx.provenance.hop_count(&batch_id(&ctx.e, 1)), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_register_duplicate_batch_fails() {
    let ctx = setup();
    register_batch(&ctx, 1);
    register_batch(&ctx, 1);
}

#[test]
fn test_transfer_custody_appends_hop_and_updates_custodian() {
    let ctx = setup();
    register_batch(&ctx, 1);

    let next = Address::generate(&ctx.e);
    ctx.provenance
        .transfer_custody(&batch_id(&ctx.e, 1), &ctx.certifier, &next);

    let batch = ctx.provenance.get_batch(&batch_id(&ctx.e, 1));
    assert_eq!(batch.custodian, next);
    assert_eq!(batch.hops, 1);

    let chain = ctx
        .provenance
        .get_custody_chain(&batch_id(&ctx.e, 1), &0, &10);
    assert_eq!(chain.len(), 1);
    let hop: CustodyHop = chain.get(0).unwrap();
    assert_eq!(hop.from, ctx.certifier);
    assert_eq!(hop.to, next);
    assert_eq!(hop.at, ctx.e.ledger().timestamp());
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_transfer_by_non_custodian_fails() {
    let ctx = setup();
    register_batch(&ctx, 1);

    let impostor = Address::generate(&ctx.e);
    let next = Address::generate(&ctx.e);
    ctx.provenance
        .transfer_custody(&batch_id(&ctx.e, 1), &impostor, &next);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_self_transfer_fails() {
    let ctx = setup();
    register_batch(&ctx, 1);
    ctx.provenance
        .transfer_custody(&batch_id(&ctx.e, 1), &ctx.certifier, &ctx.certifier);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_transfer_after_seal_fails() {
    let ctx = setup();
    register_batch(&ctx, 1);
    attach_valid_certificate(&ctx, 1, "origin-cert");
    ctx.provenance
        .seal_batch(&batch_id(&ctx.e, 1), &ctx.certifier);

    let next = Address::generate(&ctx.e);
    ctx.provenance
        .transfer_custody(&batch_id(&ctx.e, 1), &ctx.certifier, &next);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_seal_requires_current_custodian() {
    let ctx = setup();
    register_batch(&ctx, 1);
    attach_valid_certificate(&ctx, 1, "origin-cert");

    let impostor = Address::generate(&ctx.e);
    ctx.provenance.seal_batch(&batch_id(&ctx.e, 1), &impostor);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_seal_without_certificate_fails() {
    let ctx = setup();
    register_batch(&ctx, 1);
    ctx.provenance
        .seal_batch(&batch_id(&ctx.e, 1), &ctx.certifier);
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_attach_certificate_rejects_revoked_credential() {
    let ctx = setup();
    register_batch(&ctx, 1);

    let cred = credential_id(&ctx.e, "revoked-cert");
    ctx.revocation_client.revoke(&ctx.certifier, &cred);
    ctx.provenance
        .attach_certificate(&ctx.certifier, &batch_id(&ctx.e, 1), &cred);
}

#[test]
fn test_is_provenance_valid_requires_sealed_and_certificate() {
    let ctx = setup();
    register_batch(&ctx, 1);

    assert!(!ctx.provenance.is_provenance_valid(&batch_id(&ctx.e, 1)));

    attach_valid_certificate(&ctx, 1, "origin-cert");
    assert!(!ctx.provenance.is_provenance_valid(&batch_id(&ctx.e, 1)));

    ctx.provenance
        .seal_batch(&batch_id(&ctx.e, 1), &ctx.certifier);
    assert!(ctx.provenance.is_provenance_valid(&batch_id(&ctx.e, 1)));
}

#[test]
fn test_is_provenance_valid_false_after_certificate_revoked() {
    let ctx = setup();
    register_batch(&ctx, 1);

    let cred = credential_id(&ctx.e, "live-cert");
    attach_valid_certificate(&ctx, 1, "live-cert");
    ctx.provenance
        .seal_batch(&batch_id(&ctx.e, 1), &ctx.certifier);
    assert!(ctx.provenance.is_provenance_valid(&batch_id(&ctx.e, 1)));

    ctx.revocation_client.revoke(&ctx.certifier, &cred);
    assert!(!ctx.provenance.is_provenance_valid(&batch_id(&ctx.e, 1)));
}

#[test]
fn test_get_custody_chain_pagination() {
    let ctx = setup();
    register_batch(&ctx, 1);

    let mut custodian = ctx.certifier.clone();
    for i in 0..5u8 {
        let next = Address::generate(&ctx.e);
        ctx.provenance
            .transfer_custody(&batch_id(&ctx.e, 1), &custodian, &next);
        custodian = next;
        ctx.e.ledger().with_mut(|l| l.timestamp += u64::from(i + 1));
    }

    assert_eq!(ctx.provenance.hop_count(&batch_id(&ctx.e, 1)), 5);

    let page0 = ctx
        .provenance
        .get_custody_chain(&batch_id(&ctx.e, 1), &0, &2);
    assert_eq!(page0.len(), 2);

    let page2 = ctx
        .provenance
        .get_custody_chain(&batch_id(&ctx.e, 1), &2, &3);
    assert_eq!(page2.len(), 3);

    let tail = ctx
        .provenance
        .get_custody_chain(&batch_id(&ctx.e, 1), &4, &10);
    assert_eq!(tail.len(), 1);

    let empty = ctx
        .provenance
        .get_custody_chain(&batch_id(&ctx.e, 1), &10, &10);
    assert_eq!(empty.len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_get_custody_chain_rejects_oversized_limit() {
    let ctx = setup();
    register_batch(&ctx, 1);
    ctx.provenance
        .get_custody_chain(&batch_id(&ctx.e, 1), &0, &51);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_hop_limit_exceeded() {
    let ctx = setup();
    register_batch(&ctx, 1);

    let mut custodian = ctx.certifier.clone();
    for _ in 0..100 {
        let next = Address::generate(&ctx.e);
        ctx.provenance
            .transfer_custody(&batch_id(&ctx.e, 1), &custodian, &next);
        custodian = next;
    }

    let next = Address::generate(&ctx.e);
    ctx.provenance
        .transfer_custody(&batch_id(&ctx.e, 1), &custodian, &next);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice_fails() {
    let ctx = setup();
    let registry = Address::generate(&ctx.e);
    ctx.provenance.initialize(&ctx.admin, &registry);
}
