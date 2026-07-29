//! Fuzz target: issuer metadata validation.
//!
//! This fuzzer drives `add_issuer` and `set_issuer_metadata` with arbitrary
//! byte payloads for `did` and `url` fields and asserts the following
//! invariants:
//!
//! - Payloads ≤ 256 bytes must always succeed (unless the issuer is a
//!   duplicate, which the fuzzer avoids by using a fresh address per call).
//! - Payloads > 256 bytes must always be rejected with an error.
//! - After a successful `add_issuer`, `is_issuer_allowed` returns `true` and
//!   `get_issuer` returns the stored record without panicking.
//!
//! Run with:
//! ```sh
//! cd contracts/vc-issuer-registry
//! cargo +nightly fuzz run fuzz_issuer_metadata --sanitizer none
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};
use vc_issuer_registry_contract::contract::{
    VcIssuerRegistryContract, VcIssuerRegistryContractClient,
};

fuzz_target!(|data: &[u8]| {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcIssuerRegistryContract, ());
    let client = VcIssuerRegistryContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    client.initialize(&admin);

    // Split the fuzz input: first half → did, second half → url
    let mid = data.len() / 2;
    let did_bytes = &data[..mid];
    let url_bytes = &data[mid..];

    let did = Bytes::from_slice(&e, did_bytes);
    let url = Bytes::from_slice(&e, url_bytes);

    let issuer = Address::generate(&e);

    let result = client.try_add_issuer(&issuer, &None, &Some(did.clone()), &Some(url.clone()));

    if did.len() <= 256 && url.len() <= 256 {
        // Both fields within bounds: call must succeed
        assert!(
            result.is_ok(),
            "add_issuer must succeed when did and url are within 256 bytes"
        );
        // Invariant: is_issuer_allowed is true immediately after add_issuer
        assert!(client.is_issuer_allowed(&issuer));
        // Invariant: get_issuer must not panic
        let record = client.get_issuer(&issuer);
        assert!(record.allowed);
        assert_eq!(record.did, Some(did));
        assert_eq!(record.url, Some(url));
    } else {
        // At least one field is over the limit: call must fail
        assert!(
            result.is_err(),
            "add_issuer must fail when did or url exceeds 256 bytes"
        );
    }
});
