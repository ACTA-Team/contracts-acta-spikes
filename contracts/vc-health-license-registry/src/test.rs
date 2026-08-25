extern crate std;

use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Bytes, Env, Symbol};

use vc_issuer_registry_contract::contract::{
    VcIssuerRegistryContract, VcIssuerRegistryContractClient,
};

use crate::contract::{
    LicenseStatus, VcHealthLicenseRegistryContract, VcHealthLicenseRegistryContractClient,
};

struct TestEnv<'a> {
    e: Env,
    admin: Address,
    authority: Address,
    holder: Address,
    other: Address,
    license: VcHealthLicenseRegistryContractClient<'a>,
}

const BASE_TS: u64 = 1_700_000_000;

fn setup() -> TestEnv<'static> {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = BASE_TS);

    let admin = Address::generate(&e);
    let authority = Address::generate(&e);
    let holder = Address::generate(&e);
    let other = Address::generate(&e);

    let issuer_reg_id = e.register(VcIssuerRegistryContract, ());
    let license_reg_id = e.register(VcHealthLicenseRegistryContract, ());

    let issuer_client = VcIssuerRegistryContractClient::new(&e, &issuer_reg_id);
    let license = VcHealthLicenseRegistryContractClient::new(&e, &license_reg_id);

    issuer_client.initialize(&admin);
    license.initialize(&admin, &issuer_reg_id);
    issuer_client.add_issuer(&authority, &None, &None, &None);

    TestEnv {
        e,
        admin,
        authority,
        holder,
        other,
        license,
    }
}

fn setup_without_allowed_authority() -> TestEnv<'static> {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = BASE_TS);

    let admin = Address::generate(&e);
    let authority = Address::generate(&e);
    let holder = Address::generate(&e);
    let other = Address::generate(&e);

    let issuer_reg_id = e.register(VcIssuerRegistryContract, ());
    let license_reg_id = e.register(VcHealthLicenseRegistryContract, ());

    let issuer_client = VcIssuerRegistryContractClient::new(&e, &issuer_reg_id);
    let license = VcHealthLicenseRegistryContractClient::new(&e, &license_reg_id);

    issuer_client.initialize(&admin);
    license.initialize(&admin, &issuer_reg_id);

    TestEnv {
        e,
        admin,
        authority,
        holder,
        other,
        license,
    }
}

fn specialty(e: &Env, name: &str) -> Symbol {
    Symbol::new(e, name)
}

fn jurisdiction(e: &Env, name: &str) -> Symbol {
    Symbol::new(e, name)
}

fn metadata(e: &Env, suffix: &str) -> Bytes {
    Bytes::from_slice(e, suffix.as_bytes())
}

fn issue_defaults(ctx: &TestEnv) -> soroban_sdk::BytesN<32> {
    ctx.license.issue_license(
        &ctx.authority,
        &ctx.holder,
        &specialty(&ctx.e, "Cardiology"),
        &jurisdiction(&ctx.e, "US_CA"),
        &(BASE_TS + 86_400),
        &metadata(&ctx.e, "license-meta"),
    )
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_issue_license_requires_allowed_authority() {
    let ctx = setup_without_allowed_authority();
    let _ = ctx.license.issue_license(
        &ctx.authority,
        &ctx.holder,
        &specialty(&ctx.e, "Cardiology"),
        &jurisdiction(&ctx.e, "US_CA"),
        &(BASE_TS + 86_400),
        &metadata(&ctx.e, "license-meta"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_issue_license_rejects_expiry_in_past() {
    let ctx = setup();
    let _ = ctx.license.issue_license(
        &ctx.authority,
        &ctx.holder,
        &specialty(&ctx.e, "Cardiology"),
        &jurisdiction(&ctx.e, "US_CA"),
        &BASE_TS,
        &metadata(&ctx.e, "license-meta"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_duplicate_issue_fails() {
    let ctx = setup();
    let _ = issue_defaults(&ctx);
    let _ = ctx.license.issue_license(
        &ctx.authority,
        &ctx.holder,
        &specialty(&ctx.e, "Cardiology"),
        &jurisdiction(&ctx.e, "US_CA"),
        &(BASE_TS + 172_800),
        &metadata(&ctx.e, "license-meta-2"),
    );
}

#[test]
fn test_renew_extends_expiry() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);
    let new_expires = BASE_TS + 172_800;

    ctx.license
        .renew_license(&ctx.authority, &license_id, &new_expires);

    let record = ctx.license.get_license(&license_id);
    assert_eq!(record.expires_at, new_expires);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_renew_rejects_non_monotonic_expiry() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);
    ctx.license
        .renew_license(&ctx.authority, &license_id, &(BASE_TS + 86_400));
}

#[test]
fn test_suspend_then_status_is_suspended() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);
    let until = BASE_TS + 3_600;

    ctx.license.suspend_license(
        &ctx.authority,
        &license_id,
        &until,
        &Symbol::new(&ctx.e, "Investigation"),
    );

    assert_eq!(
        ctx.license.license_status(&license_id),
        LicenseStatus::Suspended
    );
}

#[test]
fn test_suspension_expires_without_transaction() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);
    let until = BASE_TS + 3_600;

    ctx.license.suspend_license(
        &ctx.authority,
        &license_id,
        &until,
        &Symbol::new(&ctx.e, "Investigation"),
    );
    assert_eq!(
        ctx.license.license_status(&license_id),
        LicenseStatus::Suspended
    );

    ctx.e.ledger().with_mut(|l| l.timestamp = until + 1);

    assert_eq!(
        ctx.license.license_status(&license_id),
        LicenseStatus::Active
    );
    assert!(ctx.license.is_license_valid(&license_id));
}

#[test]
fn test_lift_suspension_restores_active() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);
    let until = BASE_TS + 3_600;

    ctx.license.suspend_license(
        &ctx.authority,
        &license_id,
        &until,
        &Symbol::new(&ctx.e, "Investigation"),
    );
    ctx.license.lift_suspension(&ctx.authority, &license_id);

    assert_eq!(
        ctx.license.license_status(&license_id),
        LicenseStatus::Active
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_lift_suspension_when_not_suspended_fails() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);
    ctx.license.lift_suspension(&ctx.authority, &license_id);
}

#[test]
fn test_revoke_is_terminal() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);

    ctx.license.revoke_license(&ctx.authority, &license_id);
    assert_eq!(
        ctx.license.license_status(&license_id),
        LicenseStatus::Revoked
    );
    assert!(!ctx.license.is_license_valid(&license_id));

    ctx.e
        .ledger()
        .with_mut(|l| l.timestamp = BASE_TS + 86_400 * 365);

    assert_eq!(
        ctx.license.license_status(&license_id),
        LicenseStatus::Revoked
    );
}

#[test]
fn test_status_expired_after_deadline() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);

    ctx.e
        .ledger()
        .with_mut(|l| l.timestamp = BASE_TS + 86_400 + 1);

    assert_eq!(
        ctx.license.license_status(&license_id),
        LicenseStatus::Expired
    );
    assert!(!ctx.license.is_license_valid(&license_id));
}

#[test]
fn test_license_id_is_deterministic() {
    let ctx = setup();
    let spec = specialty(&ctx.e, "Cardiology");
    let juris = jurisdiction(&ctx.e, "US_CA");

    let id_a = ctx
        .license
        .license_id(&ctx.authority, &ctx.holder, &spec, &juris);
    let id_b = ctx
        .license
        .license_id(&ctx.authority, &ctx.holder, &spec, &juris);
    assert_eq!(id_a, id_b);

    let other_holder = Address::generate(&ctx.e);
    let id_c = ctx
        .license
        .license_id(&ctx.authority, &other_holder, &spec, &juris);
    assert_ne!(id_a, id_c);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_non_authority_cannot_modify() {
    let ctx = setup();
    let license_id = issue_defaults(&ctx);
    ctx.license
        .renew_license(&ctx.other, &license_id, &(BASE_TS + 172_800));
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice_fails() {
    let ctx = setup();
    let registry = Address::generate(&ctx.e);
    ctx.license.initialize(&ctx.admin, &registry);
}
