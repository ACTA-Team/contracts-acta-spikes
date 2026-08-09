#![cfg(test)]

extern crate std;

use crate::contract::{VcRevocationRegistryContract, VcRevocationRegistryContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Bytes, Env,
};

fn setup() -> (Env, VcRevocationRegistryContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    // Default ledger timestamp is 0; use a realistic close time so that
    // `revoked_at` reflects an actual ledger clock.
    e.ledger().with_mut(|l| l.timestamp = 1_700_000_000);
    let contract_id = e.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&e, &contract_id);
    (e, client)
}

// ---------------------------------------------------------------------------
// test_initialize
// ---------------------------------------------------------------------------
#[test]
fn test_initialize() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);

    assert_eq!(client.admin(), admin);
    let version = client.version();
    assert!(!version.is_empty());
}

// ---------------------------------------------------------------------------
// test_initialize_already_initialized
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_initialize_already_initialized() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);
    client.initialize(&admin); // must panic
}

// ---------------------------------------------------------------------------
// test_revoke_and_is_revoked
// ---------------------------------------------------------------------------
#[test]
fn test_revoke_and_is_revoked() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    assert!(!client.is_revoked(&issuer, &credential_id));

    client.revoke(&issuer, &credential_id);

    assert!(client.is_revoked(&issuer, &credential_id));
}

// ---------------------------------------------------------------------------
// test_revoke_already_revoked
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_revoke_already_revoked() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer, &credential_id);
    client.revoke(&issuer, &credential_id); // must panic
}

// ---------------------------------------------------------------------------
// test_unrevoke
// ---------------------------------------------------------------------------
#[test]
fn test_unrevoke() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer, &credential_id);
    assert!(client.is_revoked(&issuer, &credential_id));

    client.unrevoke(&issuer, &credential_id);
    assert!(!client.is_revoked(&issuer, &credential_id));
}

// ---------------------------------------------------------------------------
// test_unrevoke_not_revoked
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_unrevoke_not_revoked() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.unrevoke(&issuer, &credential_id); // must panic
}

// ---------------------------------------------------------------------------
// test_get_revocation
// ---------------------------------------------------------------------------
#[test]
fn test_get_revocation() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer, &credential_id);

    let record = client.get_revocation(&issuer, &credential_id);
    assert!(record.revoked_at > 0);
}

// ---------------------------------------------------------------------------
// test_get_revocation_not_found
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_get_revocation_not_found() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.get_revocation(&issuer, &credential_id); // must panic
}

// ---------------------------------------------------------------------------
// test_multiple_credentials_per_issuer
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_credentials_per_issuer() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let cred1 = Bytes::from_slice(&e, b"cred-1");
    let cred2 = Bytes::from_slice(&e, b"cred-2");

    client.revoke(&issuer, &cred1);
    assert!(client.is_revoked(&issuer, &cred1));
    assert!(!client.is_revoked(&issuer, &cred2));

    client.revoke(&issuer, &cred2);
    assert!(client.is_revoked(&issuer, &cred1));
    assert!(client.is_revoked(&issuer, &cred2));

    client.unrevoke(&issuer, &cred1);
    assert!(!client.is_revoked(&issuer, &cred1));
    assert!(client.is_revoked(&issuer, &cred2));
}

// ---------------------------------------------------------------------------
// test_multiple_issuers_same_credential_id
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_issuers_same_credential_id() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);
    let cred_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer1, &cred_id);
    assert!(client.is_revoked(&issuer1, &cred_id));
    assert!(!client.is_revoked(&issuer2, &cred_id));

    client.revoke(&issuer2, &cred_id);
    assert!(client.is_revoked(&issuer1, &cred_id));
    assert!(client.is_revoked(&issuer2, &cred_id));
}

// ---------------------------------------------------------------------------
// test_revoke_not_initialized
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_revoke_not_initialized() {
    let (e, client) = setup();

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer, &credential_id); // must panic
}

// ---------------------------------------------------------------------------
// test_admin_not_initialized
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_admin_not_initialized() {
    let (_e, client) = setup();

    client.admin(); // must panic
}

// ---------------------------------------------------------------------------
// Boundary-value tests for credential_id size
// ---------------------------------------------------------------------------

/// Exactly 256 bytes must be accepted (boundary: at the limit).
#[test]
fn test_credential_id_boundary_exact_256_accepted() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, &[b'x'; 256]);

    client.revoke(&issuer, &credential_id);
    assert!(client.is_revoked(&issuer, &credential_id));
}

/// 257 bytes must be rejected (one over the limit).
#[test]
#[should_panic]
fn test_credential_id_boundary_257_rejected() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, &[0u8; 257]);

    client.revoke(&issuer, &credential_id); // must panic with InvalidCredentialId
}

/// Empty (0-byte) credential_id must be accepted (no lower bound exists in the
/// contract logic; documenting the current behaviour explicitly so any future
/// change to reject empty ids shows up as a test failure).
#[test]
fn test_credential_id_empty_accepted() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"");

    client.revoke(&issuer, &credential_id);
    assert!(client.is_revoked(&issuer, &credential_id));
}

/// Revoke → unrevoke → revoke round-trip must work correctly.
#[test]
fn test_revoke_unrevoke_revoke_round_trip() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-roundtrip");

    // First revocation
    client.revoke(&issuer, &credential_id);
    assert!(client.is_revoked(&issuer, &credential_id));

    // Unrevoke
    client.unrevoke(&issuer, &credential_id);
    assert!(!client.is_revoked(&issuer, &credential_id));

    // Second revocation — must succeed
    client.revoke(&issuer, &credential_id);
    assert!(client.is_revoked(&issuer, &credential_id));
}

/// IDs that differ only in trailing bytes must be tracked independently.
#[test]
fn test_credential_ids_differing_only_in_trailing_bytes_are_independent() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    // Two ids that share a prefix but differ in the last byte
    let cred_a = Bytes::from_slice(&e, &[0xAA; 10]);
    let mut bytes_b = [0xAA; 10];
    bytes_b[9] = 0xBB;
    let cred_b = Bytes::from_slice(&e, &bytes_b);

    client.revoke(&issuer, &cred_a);
    assert!(client.is_revoked(&issuer, &cred_a));
    assert!(!client.is_revoked(&issuer, &cred_b));
}

// ---------------------------------------------------------------------------
// test_is_revoked_is_read_only
//   — Read-only getters must not extend instance TTL or write ledger entries.
// ---------------------------------------------------------------------------
#[test]
fn test_is_revoked_is_read_only() {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcRevocationRegistryContract, ());
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-001");

    client.initialize(&admin);
    client.revoke(&issuer, &credential_id);

    client.is_revoked(&issuer, &credential_id);

    let resources = e.cost_estimate().resources();
    assert_eq!(resources.write_entries, 0);
    assert_eq!(resources.persistent_entry_rent_bumps, 0);
}
