#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};

use crate::contract::{DidStellarRegistryContract, DidStellarRegistryContractClient};

fn setup() -> (Env, DidStellarRegistryContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(DidStellarRegistryContract, ());
    let client = DidStellarRegistryContractClient::new(&e, &contract_id);
    (e, client)
}

fn sample_document(e: &Env) -> Bytes {
    Bytes::from_slice(
        e,
        br#"{"@context":"https://www.w3.org/ns/did/v1","id":"did:stellar:test","verificationMethod":[]}"#,
    )
}

fn document_of_len(e: &Env, len: usize) -> Bytes {
    Bytes::from_slice(e, &std::vec![b'x'; len])
}

// ---------------------------------------------------------------------------
// test_register_resolve_round_trip
// ---------------------------------------------------------------------------
#[test]
fn test_register_resolve_round_trip() {
    let (e, client) = setup();
    let controller = Address::generate(&e);
    let doc = sample_document(&e);

    let did = client.register(&controller, &doc);
    let record = client.resolve(&did);

    assert_eq!(record.controller, controller);
    assert_eq!(record.document, doc);
    assert_eq!(record.version, 1);
    assert!(!record.deactivated);
    assert_eq!(record.created_at, record.updated_at);
}

// ---------------------------------------------------------------------------
// test_did_derivation_is_deterministic
//   — did_for() returns the same identifier as the one returned by register().
// ---------------------------------------------------------------------------
#[test]
fn test_did_derivation_is_deterministic() {
    let (e, client) = setup();
    let controller = Address::generate(&e);
    let doc = sample_document(&e);

    let registered_did = client.register(&controller, &doc);
    let computed_did = client.did_for(&controller);

    assert_eq!(registered_did, computed_did);
}

// ---------------------------------------------------------------------------
// test_update_bumps_version_and_timestamp
// ---------------------------------------------------------------------------
#[test]
fn test_update_bumps_version_and_timestamp() {
    let (e, client) = setup();
    let controller = Address::generate(&e);

    e.ledger().with_mut(|l| l.timestamp = 100);
    let did = client.register(&controller, &sample_document(&e));
    let before = client.resolve(&did);

    e.ledger().with_mut(|l| l.timestamp = 200);
    let new_doc = Bytes::from_slice(&e, br#"{"@context":"https://www.w3.org/ns/did/v1","id":"did:stellar:updated"}"#);
    client.update(&controller, &new_doc);

    let after = client.resolve(&did);
    assert_eq!(after.version, 2);
    assert!(after.updated_at > before.updated_at);
    assert_eq!(after.document, new_doc);
}

// ---------------------------------------------------------------------------
// test_update_by_non_controller_fails
// ---------------------------------------------------------------------------
#[test]
fn test_update_by_non_controller_fails() {
    let (e, client) = setup();
    let controller = Address::generate(&e);
    let stranger = Address::generate(&e);

    client.register(&controller, &sample_document(&e));

    // Strip all mocks so the stranger's auth is not satisfied.
    e.mock_auths(&[]);
    let new_doc = Bytes::from_slice(&e, b"{}");
    let result = client.try_update(&stranger, &new_doc);
    assert!(result.is_err(), "non-controller update must fail auth");
}

// ---------------------------------------------------------------------------
// test_deactivate_keeps_record_readable
// ---------------------------------------------------------------------------
#[test]
fn test_deactivate_keeps_record_readable() {
    let (e, client) = setup();
    let controller = Address::generate(&e);
    let doc = sample_document(&e);

    let did = client.register(&controller, &doc);
    client.deactivate(&controller);

    let record = client.resolve(&did);
    assert!(record.deactivated);
    assert_eq!(record.controller, controller);
    assert_eq!(record.document, doc);
}

// ---------------------------------------------------------------------------
// test_is_active_false_after_deactivate
// ---------------------------------------------------------------------------
#[test]
fn test_is_active_false_after_deactivate() {
    let (e, client) = setup();
    let controller = Address::generate(&e);

    let did = client.register(&controller, &sample_document(&e));
    assert!(client.is_active(&did));

    client.deactivate(&controller);
    assert!(!client.is_active(&did));
}

// ---------------------------------------------------------------------------
// test_update_after_deactivate_fails
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_update_after_deactivate_fails() {
    let (e, client) = setup();
    let controller = Address::generate(&e);

    client.register(&controller, &sample_document(&e));
    client.deactivate(&controller);
    client.update(&controller, &Bytes::from_slice(&e, b"{}"));
}

// ---------------------------------------------------------------------------
// test_register_duplicate_fails
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_register_duplicate_fails() {
    let (e, client) = setup();
    let controller = Address::generate(&e);

    client.register(&controller, &sample_document(&e));
    client.register(&controller, &sample_document(&e)); // must panic
}

// ---------------------------------------------------------------------------
// test_register_document_too_large
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_register_document_too_large() {
    let (e, client) = setup();
    let controller = Address::generate(&e);
    let oversized = document_of_len(&e, 1025);

    client.register(&controller, &oversized); // must panic
}

// ---------------------------------------------------------------------------
// test_update_document_too_large
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_update_document_too_large() {
    let (e, client) = setup();
    let controller = Address::generate(&e);

    client.register(&controller, &sample_document(&e));
    client.update(&controller, &document_of_len(&e, 1025)); // must panic
}

// ---------------------------------------------------------------------------
// test_register_document_boundary_ok
//   — A document of exactly 1024 bytes must be accepted.
// ---------------------------------------------------------------------------
#[test]
fn test_register_document_boundary_ok() {
    let (e, client) = setup();
    let controller = Address::generate(&e);
    let exact = document_of_len(&e, 1024);

    let did = client.register(&controller, &exact);
    let record = client.resolve(&did);
    assert_eq!(record.document.len(), 1024);
}

// ---------------------------------------------------------------------------
// test_resolve_unknown_did_panics
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_resolve_unknown_did_panics() {
    let (e, client) = setup();
    let fake = Bytes::from_slice(&e, &[0u8; 32]);
    client.resolve(&fake); // must panic with DidNotFound
}

// ---------------------------------------------------------------------------
// test_is_active_unknown_returns_false
// ---------------------------------------------------------------------------
#[test]
fn test_is_active_unknown_returns_false() {
    let (e, client) = setup();
    let fake = Bytes::from_slice(&e, &[0u8; 32]);
    assert!(!client.is_active(&fake));
}

// ---------------------------------------------------------------------------
// test_deactivate_by_non_controller_fails
// ---------------------------------------------------------------------------
#[test]
fn test_deactivate_by_non_controller_fails() {
    let (e, client) = setup();
    let controller = Address::generate(&e);
    let stranger = Address::generate(&e);

    client.register(&controller, &sample_document(&e));

    e.mock_auths(&[]);
    let result = client.try_deactivate(&stranger);
    assert!(result.is_err(), "non-controller deactivate must fail auth");
}

// ---------------------------------------------------------------------------
// test_distinct_controllers_distinct_dids
//   — Two different controllers get different DID identifiers.
// ---------------------------------------------------------------------------
#[test]
fn test_distinct_controllers_distinct_dids() {
    let (e, client) = setup();
    let controller_a = Address::generate(&e);
    let controller_b = Address::generate(&e);
    let doc = sample_document(&e);

    let did_a = client.register(&controller_a, &doc);
    let did_b = client.register(&controller_b, &doc);

    assert_ne!(did_a, did_b);
}
