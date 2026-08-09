//! Property-based tests for the VcIssuerRegistry state-machine invariants.
//!
//! These tests use [`proptest`] to generate random sequences of operations and
//! assert that key invariants hold regardless of the order in which operations
//! are applied:
//!
//! 1. `is_issuer_allowed == true` ⟹ `get_issuer` succeeds and returns a record
//!    with `allowed == true`.
//! 2. After `remove_issuer`, `is_issuer_allowed` is always `false`.
//! 3. `add_issuer` after `remove_issuer` succeeds and resets `allowed` to `true`.
//! 4. `set_issuer_metadata` never changes the `allowed` flag.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};

use crate::contract::{VcIssuerRegistryContract, VcIssuerRegistryContractClient};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_bytes(e: &Env, data: &[u8]) -> Bytes {
    Bytes::from_slice(e, data)
}

/// Arbitrarily-sized byte payload (0..=256), used to generate `did`/`url`
/// values that are always within the accepted range.
fn valid_payload() -> impl Strategy<Value = std::vec::Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=256)
}

// ---------------------------------------------------------------------------
// Proptest: is_issuer_allowed == true implies get_issuer does not panic
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_allowed_true_implies_get_issuer_succeeds(
        did_bytes in valid_payload(),
        url_bytes in valid_payload(),
    ) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register(VcIssuerRegistryContract, ());
        let client = VcIssuerRegistryContractClient::new(&e, &contract_id);

        let admin = Address::generate(&e);
        let issuer = Address::generate(&e);
        client.initialize(&admin);

        let did = make_bytes(&e, &did_bytes);
        let url = make_bytes(&e, &url_bytes);
        client.add_issuer(&issuer, &None, &Some(did), &Some(url));

        // Invariant: if allowed is true, get_issuer must succeed
        if client.is_issuer_allowed(&issuer) {
            let record = client.get_issuer(&issuer);
            prop_assert!(record.allowed);
        }
    }
}

// ---------------------------------------------------------------------------
// Proptest: after remove_issuer, is_issuer_allowed is always false
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_after_remove_issuer_allowed_is_false(
        did_bytes in valid_payload(),
    ) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register(VcIssuerRegistryContract, ());
        let client = VcIssuerRegistryContractClient::new(&e, &contract_id);

        let admin = Address::generate(&e);
        let issuer = Address::generate(&e);
        client.initialize(&admin);

        let did = make_bytes(&e, &did_bytes);
        client.add_issuer(&issuer, &None, &Some(did), &None);

        client.remove_issuer(&issuer);

        prop_assert!(!client.is_issuer_allowed(&issuer));
    }
}

// ---------------------------------------------------------------------------
// Proptest: add_issuer after remove_issuer succeeds and allowed == true
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_add_after_remove_succeeds_and_resets_allowed(
        did_bytes in valid_payload(),
    ) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register(VcIssuerRegistryContract, ());
        let client = VcIssuerRegistryContractClient::new(&e, &contract_id);

        let admin = Address::generate(&e);
        let issuer = Address::generate(&e);
        client.initialize(&admin);

        // First add + remove
        let did = make_bytes(&e, &did_bytes);
        client.add_issuer(&issuer, &None, &Some(did), &None);
        client.remove_issuer(&issuer);

        // Re-add
        client.add_issuer(&issuer, &None, &None, &None);

        prop_assert!(client.is_issuer_allowed(&issuer));
        let record = client.get_issuer(&issuer);
        prop_assert!(record.allowed);
    }
}

// ---------------------------------------------------------------------------
// Proptest: set_issuer_metadata never changes the allowed flag
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_set_metadata_preserves_allowed_flag(
        initial_allowed in any::<bool>(),
        new_did_bytes in valid_payload(),
        new_url_bytes in valid_payload(),
    ) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register(VcIssuerRegistryContract, ());
        let client = VcIssuerRegistryContractClient::new(&e, &contract_id);

        let admin = Address::generate(&e);
        let issuer = Address::generate(&e);
        client.initialize(&admin);

        client.add_issuer(&issuer, &None, &None, &None);

        // Set the initial allowed state
        client.set_issuer_allowed(&issuer, &initial_allowed);
        prop_assert_eq!(client.is_issuer_allowed(&issuer), initial_allowed);

        // Update metadata
        let did = make_bytes(&e, &new_did_bytes);
        let url = make_bytes(&e, &new_url_bytes);
        client.set_issuer_metadata(&issuer, &None, &Some(did), &Some(url));

        // allowed must be unchanged
        prop_assert_eq!(client.is_issuer_allowed(&issuer), initial_allowed);
        let record = client.get_issuer(&issuer);
        prop_assert_eq!(record.allowed, initial_allowed);
    }
}

// ---------------------------------------------------------------------------
// Proptest: metadata fields exceeding 256 bytes are always rejected
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_metadata_exceeding_256_bytes_rejected(
        extra_bytes in 1usize..=256usize,
    ) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register(VcIssuerRegistryContract, ());
        let client = VcIssuerRegistryContractClient::new(&e, &contract_id);

        let admin = Address::generate(&e);
        let issuer = Address::generate(&e);
        client.initialize(&admin);

        // 256 + extra_bytes overflows the limit
        let too_long = std::vec![0u8; 256 + extra_bytes];
        let did = make_bytes(&e, &too_long);

        let result = client.try_add_issuer(&issuer, &None, &Some(did), &None);
        prop_assert!(result.is_err(), "did exceeding 256 bytes must be rejected");
    }
}

// ---------------------------------------------------------------------------
// Proptest: random add/disable/enable sequence — allowed flag always consistent
// ---------------------------------------------------------------------------
/// Operation enum for state-machine sequences.
#[derive(Debug, Clone)]
enum IssuerOp {
    Disable,
    Enable,
    UpdateMetadata(std::vec::Vec<u8>),
}

fn issuer_op_strategy() -> impl Strategy<Value = IssuerOp> {
    prop_oneof![
        Just(IssuerOp::Disable),
        Just(IssuerOp::Enable),
        valid_payload().prop_map(IssuerOp::UpdateMetadata),
    ]
}

proptest! {
    #[test]
    fn prop_state_machine_allowed_flag_consistent(
        ops in prop::collection::vec(issuer_op_strategy(), 0..20),
    ) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register(VcIssuerRegistryContract, ());
        let client = VcIssuerRegistryContractClient::new(&e, &contract_id);

        let admin = Address::generate(&e);
        let issuer = Address::generate(&e);
        client.initialize(&admin);
        client.add_issuer(&issuer, &None, &None, &None);

        let mut expected_allowed = true;

        for op in ops {
            match op {
                IssuerOp::Disable => {
                    client.set_issuer_allowed(&issuer, &false);
                    expected_allowed = false;
                }
                IssuerOp::Enable => {
                    client.set_issuer_allowed(&issuer, &true);
                    expected_allowed = true;
                }
                IssuerOp::UpdateMetadata(did_bytes) => {
                    let did = make_bytes(&e, &did_bytes);
                    client.set_issuer_metadata(&issuer, &None, &Some(did), &None);
                    // allowed must be unchanged by metadata update
                }
            }
        }

        // Final invariant check
        prop_assert_eq!(client.is_issuer_allowed(&issuer), expected_allowed);
        let record = client.get_issuer(&issuer);
        prop_assert_eq!(record.allowed, expected_allowed);
    }
}
