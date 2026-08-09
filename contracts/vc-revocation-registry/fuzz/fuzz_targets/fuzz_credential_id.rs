//! Fuzz target: credential-ID validation and revoke/unrevoke invariants.
//!
//! This fuzzer drives `revoke` with arbitrary byte slices as `credential_id`
//! and asserts:
//!
//! - Payloads ≤ 256 bytes must always be accepted by `revoke`.
//! - Payloads > 256 bytes must always be rejected with an error.
//! - After a successful `revoke`, `is_revoked` returns `true`.
//! - After `unrevoke`, `is_revoked` returns `false`.
//! - A second `revoke` on the same ID (after `unrevoke`) succeeds.
//!
//! Run with:
//! ```sh
//! cd contracts/vc-revocation-registry
//! cargo +nightly fuzz run fuzz_credential_id --sanitizer none
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Bytes, Env,
};
use vc_revocation_registry_contract::contract::{
    VcRevocationRegistryContract, VcRevocationRegistryContractClient,
};

fuzz_target!(|data: &[u8]| {
    let e = Env::default();
    e.mock_all_auths();
    // Default ledger timestamp is 0; `revoked_at > 0` below needs a real clock.
    e.ledger().with_mut(|l| l.timestamp = 1_700_000_000);
    let contract_id = e.register(VcRevocationRegistryContract, ());
    let client = VcRevocationRegistryContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    client.initialize(&admin);

    let issuer = Address::generate(&e);
    let credential_id = Bytes::from_slice(&e, data);

    let revoke_result = client.try_revoke(&issuer, &credential_id);

    if data.len() <= 256 {
        // Within bounds — must succeed
        assert!(
            revoke_result.is_ok(),
            "revoke must succeed for credential_id with {} bytes (≤ 256)",
            data.len()
        );

        // is_revoked must now return true
        assert!(client.is_revoked(&issuer, &credential_id));

        // get_revocation must succeed
        let record = client.get_revocation(&issuer, &credential_id);
        assert!(record.revoked_at > 0);

        // Unrevoke
        client.unrevoke(&issuer, &credential_id);
        assert!(!client.is_revoked(&issuer, &credential_id));

        // Re-revoke must succeed (round-trip)
        client.revoke(&issuer, &credential_id);
        assert!(client.is_revoked(&issuer, &credential_id));
    } else {
        // Over the limit — must fail
        assert!(
            revoke_result.is_err(),
            "revoke must fail for credential_id with {} bytes (> 256)",
            data.len()
        );
    }
});
