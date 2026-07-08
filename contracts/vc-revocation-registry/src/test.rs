#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol, Vec};

use crate::contract::{VcRevocationRegistryContract, VcRevocationRegistryContractClient};

fn setup() -> (Env, VcRevocationRegistryContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&e, &contract_id);
    (e, client)
}

fn vc_id(e: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(e, &[seed; 32])
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_sets_admin() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);

    assert_eq!(client.admin(), admin);
}

#[test]
fn test_initialize_emits_event() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);

    // Verify the Initialized event was published (env recorded it)
    assert!(!e.events().all().is_empty(), "Initialized event must be emitted");
}

#[test]
#[should_panic]
fn test_initialize_panics_when_called_twice() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);
    client.initialize(&admin); // must panic with AlreadyInitialized
}

// ---------------------------------------------------------------------------
// revoke
// ---------------------------------------------------------------------------

#[test]
fn test_revoke_happy_path() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 1);

    client.initialize(&admin);
    assert!(!client.is_revoked(&id));

    client.revoke(&issuer, &id, &None);

    assert!(client.is_revoked(&id));
}

#[test]
fn test_revoke_stores_record_fields() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 2);
    let reason = Symbol::new(&e, "Expired");

    client.initialize(&admin);
    client.revoke(&issuer, &id, &Some(reason.clone()));

    let record = client.get_revocation(&id);
    assert_eq!(record.issuer, issuer);
    assert_eq!(record.reason, Some(reason));
}

#[test]
fn test_revoke_emits_revoked_event() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 3);

    client.initialize(&admin);
    client.revoke(&issuer, &id, &None);

    assert!(!e.events().all().is_empty(), "Revoked event must be emitted");
}

#[test]
#[should_panic]
fn test_revoke_panics_when_already_revoked() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 4);

    client.initialize(&admin);
    client.revoke(&issuer, &id, &None);
    client.revoke(&issuer, &id, &None); // must panic with AlreadyRevoked
}

#[test]
#[should_panic]
fn test_revoke_panics_when_not_initialized() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 5);

    client.revoke(&issuer, &id, &None); // must panic with NotInitialized
}

#[test]
fn test_revoke_non_issuer_auth_rejected() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 6);

    client.initialize(&admin);

    e.mock_auths(&[]);
    let result = client.try_revoke(&issuer, &id, &None);
    assert!(result.is_err(), "call without auth must fail");
}

// ---------------------------------------------------------------------------
// batch_revoke
// ---------------------------------------------------------------------------

#[test]
fn test_batch_revoke_happy_path() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);

    client.initialize(&admin);

    let ids = Vec::from_array(&e, [vc_id(&e, 10), vc_id(&e, 11), vc_id(&e, 12)]);
    client.batch_revoke(&issuer, &ids, &None);

    for id in ids.iter() {
        assert!(client.is_revoked(&id));
    }
}

#[test]
fn test_batch_revoke_emits_one_event_per_vc() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);

    client.initialize(&admin);

    let ids = Vec::from_array(&e, [vc_id(&e, 20), vc_id(&e, 21)]);
    client.batch_revoke(&issuer, &ids, &None);

    // 1 Initialized + 2 Revoked events = 3 total
    assert_eq!(e.events().all().len(), 3);
}

#[test]
#[should_panic]
fn test_batch_revoke_rolls_back_if_any_already_revoked() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id1 = vc_id(&e, 30);
    let id2 = vc_id(&e, 31);

    client.initialize(&admin);
    client.revoke(&issuer, &id1, &None); // id1 already revoked

    let ids = Vec::from_array(&e, [id1, id2]);
    client.batch_revoke(&issuer, &ids, &None); // must panic with AlreadyRevoked
}

// ---------------------------------------------------------------------------
// unrevoke
// ---------------------------------------------------------------------------

#[test]
fn test_unrevoke_happy_path() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 40);

    client.initialize(&admin);
    client.revoke(&issuer, &id, &None);
    assert!(client.is_revoked(&id));

    client.unrevoke(&id);

    assert!(!client.is_revoked(&id));
}

#[test]
fn test_unrevoke_emits_event() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 41);

    client.initialize(&admin);
    client.revoke(&issuer, &id, &None);

    let events_before = e.events().all().len();
    client.unrevoke(&id);

    assert!(e.events().all().len() > events_before, "Unrevoked event must be emitted");
}

#[test]
#[should_panic]
fn test_unrevoke_panics_when_not_revoked() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let id = vc_id(&e, 42);

    client.initialize(&admin);
    client.unrevoke(&id); // must panic with NotRevoked
}

#[test]
fn test_unrevoke_non_admin_rejected() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 43);

    client.initialize(&admin);
    client.revoke(&issuer, &id, &None);

    e.mock_auths(&[]);
    let result = client.try_unrevoke(&id);
    assert!(result.is_err(), "non-admin unrevoke must fail");
}

// ---------------------------------------------------------------------------
// is_revoked / get_revocation
// ---------------------------------------------------------------------------

#[test]
fn test_is_revoked_returns_false_before_revoke() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let id = vc_id(&e, 50);

    client.initialize(&admin);
    assert!(!client.is_revoked(&id));
}

#[test]
fn test_is_revoked_returns_true_after_revoke() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 51);

    client.initialize(&admin);
    client.revoke(&issuer, &id, &None);
    assert!(client.is_revoked(&id));
}

#[test]
fn test_is_revoked_returns_false_after_unrevoke() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let id = vc_id(&e, 52);

    client.initialize(&admin);
    client.revoke(&issuer, &id, &None);
    client.unrevoke(&id);
    assert!(!client.is_revoked(&id));
}

#[test]
#[should_panic]
fn test_get_revocation_panics_when_not_revoked() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let id = vc_id(&e, 53);

    client.initialize(&admin);
    client.get_revocation(&id); // must panic with NotRevoked
}

// ---------------------------------------------------------------------------
// admin / version
// ---------------------------------------------------------------------------

#[test]
#[should_panic]
fn test_admin_panics_when_not_initialized() {
    let (_e, client) = setup();
    client.admin(); // must panic with NotInitialized
}

#[test]
fn test_version_returns_string() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);
    let v = client.version();
    assert!(!v.is_empty(), "version must return a non-empty string");
}
