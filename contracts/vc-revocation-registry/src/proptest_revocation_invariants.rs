//! Property-based tests for the VcRevocationRegistry invariants.
//!
//! Properties verified:
//!
//! 1. **Revoke → unrevoke → revoke round-trip** is well-defined for arbitrary
//!    `(issuer, credential_id)` pairs.
//! 2. **IDs differing only in trailing bytes** are tracked independently.
//! 3. **Credential IDs exceeding 256 bytes** are always rejected by `revoke`.
//! 4. **`is_revoked` is always consistent** with the most recent revoke/unrevoke
//!    operation in a random sequence.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};

use crate::contract::{VcRevocationRegistryContract, VcRevocationRegistryContractClient};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (Env, VcRevocationRegistryContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&e, &contract_id);
    (e, client)
}

fn make_bytes(e: &Env, data: &[u8]) -> Bytes {
    Bytes::from_slice(e, data)
}

/// Valid credential ID: 0..=256 bytes.
fn valid_credential_id() -> impl Strategy<Value = std::vec::Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=256)
}

// ---------------------------------------------------------------------------
// Proptest: revoke → unrevoke → revoke round-trip
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_revoke_unrevoke_revoke_round_trip(
        cred_bytes in valid_credential_id(),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let issuer = Address::generate(&e);
        let cred_id = make_bytes(&e, &cred_bytes);

        // First revocation
        client.revoke(&issuer, &cred_id);
        prop_assert!(client.is_revoked(&issuer, &cred_id));

        // Unrevoke
        client.unrevoke(&issuer, &cred_id);
        prop_assert!(!client.is_revoked(&issuer, &cred_id));

        // Second revocation — must succeed
        client.revoke(&issuer, &cred_id);
        prop_assert!(client.is_revoked(&issuer, &cred_id));
    }
}

// ---------------------------------------------------------------------------
// Proptest: IDs differing only in trailing bytes are independent
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_ids_differing_in_trailing_byte_are_independent(
        prefix in prop::collection::vec(any::<u8>(), 1..=255),
        byte_a in any::<u8>(),
        byte_b in any::<u8>(),
    ) {
        prop_assume!(byte_a != byte_b);

        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let issuer = Address::generate(&e);

        let mut id_a_bytes = prefix.clone();
        id_a_bytes.push(byte_a);
        let mut id_b_bytes = prefix.clone();
        id_b_bytes.push(byte_b);

        let cred_a = make_bytes(&e, &id_a_bytes);
        let cred_b = make_bytes(&e, &id_b_bytes);

        // Revoke only cred_a
        client.revoke(&issuer, &cred_a);

        prop_assert!(client.is_revoked(&issuer, &cred_a), "cred_a should be revoked");
        prop_assert!(!client.is_revoked(&issuer, &cred_b), "cred_b must not be revoked");
    }
}

// ---------------------------------------------------------------------------
// Proptest: credential IDs exceeding 256 bytes are always rejected
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_credential_id_exceeding_256_bytes_rejected(
        extra in 1usize..=256usize,
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let issuer = Address::generate(&e);
        let too_long = std::vec![0u8; 256 + extra];
        let cred_id = make_bytes(&e, &too_long);

        let result = client.try_revoke(&issuer, &cred_id);
        prop_assert!(result.is_err(), "credential_id exceeding 256 bytes must be rejected");
    }
}

// ---------------------------------------------------------------------------
// Proptest: is_revoked is consistent after random revoke/unrevoke sequences
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
enum RevocationOp {
    Revoke,
    Unrevoke,
}

fn revocation_op_strategy() -> impl Strategy<Value = RevocationOp> {
    prop_oneof![Just(RevocationOp::Revoke), Just(RevocationOp::Unrevoke)]
}

proptest! {
    #[test]
    fn prop_is_revoked_consistent_with_operations(
        cred_bytes in valid_credential_id(),
        ops in prop::collection::vec(revocation_op_strategy(), 1..15),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let issuer = Address::generate(&e);
        let cred_id = make_bytes(&e, &cred_bytes);

        let mut is_revoked = false;

        for op in &ops {
            match op {
                RevocationOp::Revoke => {
                    if !is_revoked {
                        client.revoke(&issuer, &cred_id);
                        is_revoked = true;
                    }
                }
                RevocationOp::Unrevoke => {
                    if is_revoked {
                        client.unrevoke(&issuer, &cred_id);
                        is_revoked = false;
                    }
                }
            }
        }

        prop_assert_eq!(
            client.is_revoked(&issuer, &cred_id),
            is_revoked,
            "is_revoked must match the tracked state after operations"
        );
    }
}

// ---------------------------------------------------------------------------
// Proptest: different issuers with the same credential_id are tracked separately
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_different_issuers_tracked_independently(
        cred_bytes in valid_credential_id(),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let issuer_a = Address::generate(&e);
        let issuer_b = Address::generate(&e);
        let cred_id = make_bytes(&e, &cred_bytes);

        // Revoke only for issuer_a
        client.revoke(&issuer_a, &cred_id);

        prop_assert!(client.is_revoked(&issuer_a, &cred_id));
        prop_assert!(!client.is_revoked(&issuer_b, &cred_id));
    }
}
