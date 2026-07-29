#![cfg(test)]

extern crate std;

use crate::contract::VcRevocationRegistryContract;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Bytes, Env, String,
};

fn setup() -> (Env, Address, Address) {
    let e = Env::default();
    e.ledger().set_timestamp(12_345);
    let admin = Address::generate(&e);
    let contract_id = e.register(VcRevocationRegistryContract, ());
    e.mock_all_auths();
    (e, contract_id, admin)
}

#[test]
fn test_initialize() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    assert_eq!(client.admin(), admin);
    let version: String = client.version();
    assert!(!version.is_empty());
}

#[test]
fn test_initialize_already_initialized() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.initialize(&admin);
    }));
    assert!(result.is_err());
}

#[test]
fn test_revoke_and_is_revoked() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    assert!(!client.is_revoked(&issuer, &credential_id));

    client.revoke(&issuer, &credential_id);

    assert!(client.is_revoked(&issuer, &credential_id));
}

#[test]
fn test_revoke_already_revoked() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer, &credential_id);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.revoke(&issuer, &credential_id);
    }));
    assert!(result.is_err());
}

#[test]
fn test_unrevoke() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer, &credential_id);
    assert!(client.is_revoked(&issuer, &credential_id));

    client.unrevoke(&issuer, &credential_id);
    assert!(!client.is_revoked(&issuer, &credential_id));
}

#[test]
fn test_unrevoke_not_revoked() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.unrevoke(&issuer, &credential_id);
    }));
    assert!(result.is_err());
}

#[test]
fn test_get_revocation() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    client.revoke(&issuer, &credential_id);

    let record = client.get_revocation(&issuer, &credential_id);
    assert!(record.revoked_at > 0);
}

#[test]
fn test_get_revocation_not_found() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.get_revocation(&issuer, &credential_id);
    }));
    assert!(result.is_err());
}

#[test]
fn test_multiple_credentials_per_issuer() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

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

#[test]
fn test_multiple_issuers_same_credential_id() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

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

#[test]
fn test_revoke_not_initialized() {
    let (e, contract_id, _admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, b"cred-123");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.revoke(&issuer, &credential_id);
    }));
    assert!(result.is_err());
}

#[test]
fn test_admin_not_initialized() {
    let (e, contract_id, _admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.admin();
    }));
    assert!(result.is_err());
}

#[test]
fn test_invalid_credential_id() {
    let (e, contract_id, admin) = setup();
    let client = crate::contract::VcRevocationRegistryContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let issuer = Address::generate(&e);
    // Credential ID exceeds 256 bytes
    let credential_id = Bytes::from_slice(&e, &[0u8; 257]);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.revoke(&issuer, &credential_id);
    }));
    assert!(result.is_err());
}
