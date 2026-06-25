//! Tests for vc-revocation-registry.

#![cfg(test)]

use crate::contract::{VcRevocationRegistryContract, VcRevocationRegistryContractClient};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol, Vec};

const VC_ID_1: [u8; 32] = [1u8; 32];
const VC_ID_2: [u8; 32] = [2u8; 32];
const VC_ID_3: [u8; 32] = [3u8; 32];

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    client.initialize(&admin);

    assert_eq!(client.admin(), admin);
}

#[test]
#[should_panic(expected = "AlreadyInitialized")]
fn test_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
fn test_revoke() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    client.initialize(&admin);
    
    let vc_id = BytesN::<32>::from_array(&env, &VC_ID_1);
    client.revoke(&issuer, &vc_id, &None);

    assert!(client.is_revoked(&vc_id));
    let record = client.get_revocation(&vc_id);
    assert_eq!(record.issuer, issuer);
    assert_eq!(record.reason, None);
}

#[test]
#[should_panic(expected = "AlreadyRevoked")]
fn test_revoke_duplicate() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    client.initialize(&admin);
    
    let vc_id = BytesN::<32>::from_array(&env, &VC_ID_1);
    client.revoke(&issuer, &vc_id, &None);
    client.revoke(&issuer, &vc_id, &None);
}

#[test]
fn test_batch_revoke() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    client.initialize(&admin);
    
    let vc_id_1 = BytesN::<32>::from_array(&env, &VC_ID_1);
    let vc_id_2 = BytesN::<32>::from_array(&env, &VC_ID_2);
    let vc_id_3 = BytesN::<32>::from_array(&env, &VC_ID_3);
    
    let mut vc_ids = Vec::new(&env);
    vc_ids.push_back(vc_id_1.clone());
    vc_ids.push_back(vc_id_2.clone());
    vc_ids.push_back(vc_id_3.clone());

    let reason = Some(Symbol::new(&env, "duplicates"));
    client.batch_revoke(&issuer, &vc_ids, &reason);

    assert!(client.is_revoked(&vc_id_1));
    assert!(client.is_revoked(&vc_id_2));
    assert!(client.is_revoked(&vc_id_3));

    let record = client.get_revocation(&vc_id_1);
    assert_eq!(record.issuer, issuer);
    assert_eq!(record.reason, Some(Symbol::new(&env, "duplicates")));
}

#[test]
fn test_unrevoke() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    client.initialize(&admin);
    
    let vc_id = BytesN::<32>::from_array(&env, &VC_ID_1);
    client.revoke(&issuer, &vc_id, &None);
    assert!(client.is_revoked(&vc_id));

    client.unrevoke(&vc_id);
    assert!(!client.is_revoked(&vc_id));
}

#[test]
#[should_panic(expected = "NotRevoked")]
fn test_unrevoke_not_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    client.initialize(&admin);
    
    let vc_id = BytesN::<32>::from_array(&env, &VC_ID_1);
    client.unrevoke(&vc_id);
}

#[test]
fn test_is_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    client.initialize(&admin);
    
    let vc_id = BytesN::<32>::from_array(&env, &VC_ID_1);
    assert!(!client.is_revoked(&vc_id));

    client.revoke(&issuer, &vc_id, &None);
    assert!(client.is_revoked(&vc_id));
}

#[test]
#[should_panic(expected = "NotInitialized")]
fn test_revoke_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let issuer = Address::generate(&env);
    let vc_id = BytesN::<32>::from_array(&env, &VC_ID_1);
    client.revoke(&issuer, &vc_id, &None);
}

#[test]
fn test_version() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    client.initialize(&admin);
    
    let version = client.version();
    // Version string contains "0.1.0"
    assert_eq!(version, soroban_sdk::String::from_str(&env, "0.1.0"));
}

#[test]
#[should_panic(expected = "NotRevoked")]
fn test_get_revocation_not_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    client.initialize(&admin);
    
    let vc_id = BytesN::<32>::from_array(&env, &VC_ID_1);
    client.get_revocation(&vc_id);
}
