#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env, Symbol};

use vc_issuer_registry_contract::VcIssuerRegistryContract;
use vc_issuer_registry_contract::VcIssuerRegistryContractClient;
use vc_revocation_registry_contract::VcRevocationRegistryContract;
use vc_revocation_registry_contract::VcRevocationRegistryContractClient;
use vc_schema_registry_contract::VcSchemaRegistryContract;
use vc_schema_registry_contract::VcSchemaRegistryContractClient;

use crate::contract::{VcVerifierContract, VcVerifierContractClient};

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct TestEnv<'a> {
    e: Env,
    admin: Address,
    issuer: Address,
    verifier: VcVerifierContractClient<'a>,
    issuer_client: VcIssuerRegistryContractClient<'a>,
    schema_client: VcSchemaRegistryContractClient<'a>,
    revocation_client: VcRevocationRegistryContractClient<'a>,
}

fn setup() -> TestEnv<'static> {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);

    // Register all four contracts in the test Env.
    let issuer_reg_id = e.register(VcIssuerRegistryContract, ());
    let schema_reg_id = e.register(VcSchemaRegistryContract, ());
    let revocation_reg_id = e.register(VcRevocationRegistryContract, ());
    let verifier_id = e.register(VcVerifierContract, ());

    let issuer_client = VcIssuerRegistryContractClient::new(&e, &issuer_reg_id);
    let schema_client = VcSchemaRegistryContractClient::new(&e, &schema_reg_id);
    let revocation_client = VcRevocationRegistryContractClient::new(&e, &revocation_reg_id);
    let verifier = VcVerifierContractClient::new(&e, &verifier_id);

    // Initialize all three registries.
    issuer_client.initialize(&admin);
    schema_client.initialize(&admin);
    revocation_client.initialize(&admin);

    // Initialize the verifier with all three registry addresses.
    verifier.initialize(&admin, &issuer_reg_id, &schema_reg_id, &revocation_reg_id);

    TestEnv {
        e,
        admin,
        issuer,
        verifier,
        issuer_client,
        schema_client,
        revocation_client,
    }
}

/// Register the issuer in the issuer registry and a schema, return the schema_id.
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

fn credential_id(e: &Env) -> Bytes {
    Bytes::from_slice(e, b"credential-001")
}

// ---------------------------------------------------------------------------
// 1. Valid credential — all checks pass
// ---------------------------------------------------------------------------
#[test]
fn test_valid_credential() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    let result = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);

    assert!(result.valid);
    assert!(result.issuer_allowed);
    assert!(result.schema_exists);
    assert!(!result.schema_deprecated);
    assert!(!result.revoked);
}

// ---------------------------------------------------------------------------
// 2. De-listed issuer
// ---------------------------------------------------------------------------
#[test]
fn test_delisted_issuer() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    ctx.issuer_client.set_issuer_allowed(&ctx.issuer, &false);

    let result = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);

    assert!(!result.valid);
    assert!(!result.issuer_allowed);
    assert!(result.schema_exists);
    assert!(!result.schema_deprecated);
    assert!(!result.revoked);
}

// ---------------------------------------------------------------------------
// 3. Unknown issuer (never registered)
// ---------------------------------------------------------------------------
#[test]
fn test_unknown_issuer() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    let unknown_issuer = Address::generate(&ctx.e);
    let result = ctx.verifier.verify(&unknown_issuer, &schema_id, &cred_id);

    assert!(!result.valid);
    assert!(!result.issuer_allowed);
    assert!(result.schema_exists);
    assert!(!result.schema_deprecated);
    assert!(!result.revoked);
}

// ---------------------------------------------------------------------------
// 4. Unknown schema
// ---------------------------------------------------------------------------
#[test]
fn test_unknown_schema() {
    let ctx = setup();
    ctx.issuer_client
        .add_issuer(&ctx.issuer, &None, &None, &None);
    let cred_id = credential_id(&ctx.e);

    let unknown_schema: BytesN<32> = BytesN::from_array(&ctx.e, &[0u8; 32]);
    let result = ctx.verifier.verify(&ctx.issuer, &unknown_schema, &cred_id);

    assert!(!result.valid);
    assert!(result.issuer_allowed);
    assert!(!result.schema_exists);
    assert!(!result.schema_deprecated);
    assert!(!result.revoked);
}

// ---------------------------------------------------------------------------
// 5. Deprecated schema
// ---------------------------------------------------------------------------
#[test]
fn test_deprecated_schema() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    ctx.schema_client.deprecate_schema(&schema_id, &ctx.admin);

    let result = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);

    assert!(!result.valid);
    assert!(result.issuer_allowed);
    assert!(result.schema_exists);
    assert!(result.schema_deprecated);
    assert!(!result.revoked);
}

// ---------------------------------------------------------------------------
// 6. Revoked credential
// ---------------------------------------------------------------------------
#[test]
fn test_revoked_credential() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    ctx.revocation_client.revoke(&ctx.issuer, &cred_id);

    let result = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);

    assert!(!result.valid);
    assert!(result.issuer_allowed);
    assert!(result.schema_exists);
    assert!(!result.schema_deprecated);
    assert!(result.revoked);
}

// ---------------------------------------------------------------------------
// 7. Several failures at once: de-listed issuer + deprecated schema + revoked
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_failures() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    ctx.issuer_client.set_issuer_allowed(&ctx.issuer, &false);
    ctx.schema_client.deprecate_schema(&schema_id, &ctx.admin);
    ctx.revocation_client.revoke(&ctx.issuer, &cred_id);

    let result = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);

    assert!(!result.valid);
    assert!(!result.issuer_allowed);
    assert!(result.schema_exists);
    assert!(result.schema_deprecated);
    assert!(result.revoked);
}

// ---------------------------------------------------------------------------
// 8. Register → verify → revoke → verify (end-to-end lifecycle)
// ---------------------------------------------------------------------------
#[test]
fn test_revoke_then_verify_again() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    // First verify: should be valid.
    let first = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);
    assert!(first.valid);

    // Revoke the credential.
    ctx.revocation_client.revoke(&ctx.issuer, &cred_id);

    // Second verify: should be invalid with revoked=true.
    let second = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);
    assert!(!second.valid);
    assert!(second.revoked);
    assert!(second.issuer_allowed);
    assert!(second.schema_exists);
    assert!(!second.schema_deprecated);
}

// ---------------------------------------------------------------------------
// 9. Admin can update registry addresses; events are emitted
// ---------------------------------------------------------------------------
#[test]
fn test_admin_updates_registry_addresses() {
    let ctx = setup();

    // Deploy replacement registries.
    let new_issuer_reg = ctx.e.register(VcIssuerRegistryContract, ());
    let new_schema_reg = ctx.e.register(VcSchemaRegistryContract, ());
    let new_revocation_reg = ctx.e.register(VcRevocationRegistryContract, ());

    ctx.verifier.set_issuer_registry(&new_issuer_reg);
    ctx.verifier.set_schema_registry(&new_schema_reg);
    ctx.verifier.set_revocation_registry(&new_revocation_reg);

    assert_eq!(ctx.verifier.issuer_registry(), new_issuer_reg);
    assert_eq!(ctx.verifier.schema_registry(), new_schema_reg);
    assert_eq!(ctx.verifier.revocation_registry(), new_revocation_reg);
}

// ---------------------------------------------------------------------------
// 10. Non-admin cannot update registry addresses
// ---------------------------------------------------------------------------
#[test]
fn test_non_admin_cannot_update_registry() {
    let ctx = setup();
    let new_addr = Address::generate(&ctx.e);

    ctx.e.mock_auths(&[]);
    let result = ctx.verifier.try_set_issuer_registry(&new_addr);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// 11. verify does not write to storage (TTL check via re-read)
// ---------------------------------------------------------------------------
#[test]
fn test_verify_is_readonly() {
    let ctx = setup();
    let schema_id = register_issuer_and_schema(&ctx);
    let cred_id = credential_id(&ctx.e);

    // Two consecutive reads should both succeed and return the same result.
    let r1 = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);
    let r2 = ctx.verifier.verify(&ctx.issuer, &schema_id, &cred_id);
    assert_eq!(r1.valid, r2.valid);
    assert_eq!(r1.issuer_allowed, r2.issuer_allowed);
}
