#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::Address as _, Address, Env, Symbol, Vec,
};

use crate::contract::{VcStatusListContract, VcStatusListContractClient};
use crate::storage;

fn setup() -> (Env, VcStatusListContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcStatusListContract, ());
    let client = VcStatusListContractClient::new(&e, &contract_id);
    (e, client)
}

fn create_test_list(
    client: &VcStatusListContractClient,
    issuer: &Address,
    list_id: &Symbol,
    size: u32,
) {
    client.create_list(issuer, list_id, &size);
}

// ---------------------------------------------------------------------------
// test_create_list_and_metadata
// ---------------------------------------------------------------------------
#[test]
fn test_create_list_and_metadata() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    client.create_list(&issuer, &list_id, &128);

    assert!(client.list_exists(&issuer, &list_id));

    let meta = client.get_list_metadata(&issuer, &list_id);
    assert_eq!(meta.size, 128);
    assert!(meta.num_chunks > 0);
}

// ---------------------------------------------------------------------------
// test_create_list_zero_size_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_create_list_zero_size_rejected() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    client.create_list(&issuer, &list_id, &0); // must panic with SizeZero
}

// ---------------------------------------------------------------------------
// test_create_list_size_too_large_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_create_list_size_too_large_rejected() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    client.create_list(&issuer, &list_id, &(storage::MAX_LIST_SIZE + 1)); // must panic
}

// ---------------------------------------------------------------------------
// test_create_list_duplicate_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_create_list_duplicate_rejected() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    client.create_list(&issuer, &list_id, &64);
    client.create_list(&issuer, &list_id, &64); // must panic with ListAlreadyExists
}

// ---------------------------------------------------------------------------
// test_set_status_and_is_revoked_round_trip
// ---------------------------------------------------------------------------
#[test]
fn test_set_status_and_is_revoked_round_trip() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 128);

    // Initially not revoked
    assert!(!client.is_revoked(&issuer, &list_id, &5));

    // Revoke
    client.set_status(&issuer, &list_id, &5, &true);
    assert!(client.is_revoked(&issuer, &list_id, &5));

    // Unrevoke
    client.set_status(&issuer, &list_id, &5, &false);
    assert!(!client.is_revoked(&issuer, &list_id, &5));
}

// ---------------------------------------------------------------------------
// test_set_status_out_of_range_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_set_status_out_of_range_rejected() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 128);

    client.set_status(&issuer, &list_id, &128, &true); // must panic: out of range
}

// ---------------------------------------------------------------------------
// test_is_revoked_out_of_range_returns_false
// ---------------------------------------------------------------------------
#[test]
fn test_is_revoked_out_of_range_returns_false() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 128);

    // Out-of-range indices should return false gracefully (verifier reads).
    assert!(!client.is_revoked(&issuer, &list_id, &128));
    assert!(!client.is_revoked(&issuer, &list_id, &9999));
}

// ---------------------------------------------------------------------------
// test_is_revoked_nonexistent_list_returns_false
// ---------------------------------------------------------------------------
#[test]
fn test_is_revoked_nonexistent_list_returns_false() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "nonexistent");

    assert!(!client.is_revoked(&issuer, &list_id, &0));
}

// ---------------------------------------------------------------------------
// test_set_status_batch
// ---------------------------------------------------------------------------
#[test]
fn test_set_status_batch() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 256);

    let indices = Vec::from_array(&e, [
        0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
        50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
        60, 61, 62, 63, 64, 65, 66, 67, 68, 69,
        70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
        80, 81, 82, 83, 84, 85, 86, 87, 88, 89,
        90, 91, 92, 93, 94, 95, 96, 97, 98, 99,
    ]);

    // None revoked initially
    assert!(!client.is_revoked(&issuer, &list_id, &0));
    assert!(!client.is_revoked(&issuer, &list_id, &50));
    assert!(!client.is_revoked(&issuer, &list_id, &99));

    // Batch revoke
    client.set_status_batch(&issuer, &list_id, &indices, &true);

    // All should now be revoked
    assert!(client.is_revoked(&issuer, &list_id, &0));
    assert!(client.is_revoked(&issuer, &list_id, &50));
    assert!(client.is_revoked(&issuer, &list_id, &99));

    // Batch unrevoke using same indices
    client.set_status_batch(&issuer, &list_id, &indices, &false);

    assert!(!client.is_revoked(&issuer, &list_id, &0));
    assert!(!client.is_revoked(&issuer, &list_id, &50));
    assert!(!client.is_revoked(&issuer, &list_id, &99));
}

// ---------------------------------------------------------------------------
// test_set_status_batch_out_of_range_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_set_status_batch_out_of_range_rejected() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 128);

    let indices = Vec::from_array(&e, [0, 5, 128]); // 128 is out of range
    client.set_status_batch(&issuer, &list_id, &indices, &true);
}

// ---------------------------------------------------------------------------
// test_set_status_on_nonexistent_list
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_set_status_on_nonexistent_list() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "nonexistent");

    client.set_status(&issuer, &list_id, &0, &true); // must panic: ListNotFound
}

// ---------------------------------------------------------------------------
// test_unauthorized_issuer_rejected
// ---------------------------------------------------------------------------
#[test]
fn test_unauthorized_issuer_rejected() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");
    create_test_list(&client, &issuer, &list_id, 128);

    // Clear mocks so no valid auth is provided for set_status
    e.mock_auths(&[]);
    let result = client.try_set_status(&issuer, &list_id, &0, &true);
    assert!(result.is_err(), "unauthorized call must fail");
}

// ---------------------------------------------------------------------------
// test_unauthorized_create_list_rejected
// ---------------------------------------------------------------------------
#[test]
fn test_unauthorized_create_list_rejected() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);

    // Clear mocks so no valid auth is provided
    e.mock_auths(&[]);
    let result = client.try_create_list(&issuer, &Symbol::new(&e, "main"), &128u32);
    assert!(result.is_err(), "unauthorized create must fail");
}

// ---------------------------------------------------------------------------
// test_get_chunk
// ---------------------------------------------------------------------------
#[test]
fn test_get_chunk() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 128);

    // Chunk 0 should exist and be readable; newly created lists have empty chunks
    let _chunk0 = client.get_chunk(&issuer, &list_id, &0);

    // Set a bit and verify it appears in the chunk bytes
    client.set_status(&issuer, &list_id, &0, &true);
    let chunk = client.get_chunk(&issuer, &list_id, &0);
    // Bit 0 is the MSB of byte 0 => mask 0x80
    assert!(chunk.len() > 0);
    assert_eq!(chunk.get_unchecked(0) & 0x80, 0x80);
}

// ---------------------------------------------------------------------------
// test_get_chunk_nonexistent_list
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_get_chunk_nonexistent_list() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "nonexistent");

    client.get_chunk(&issuer, &list_id, &0); // must panic: ListNotFound
}

// ---------------------------------------------------------------------------
// test_get_chunk_out_of_range_returns_empty
// ---------------------------------------------------------------------------
#[test]
fn test_get_chunk_out_of_range_returns_empty() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 128);

    // Chunk 999 is beyond the list — returns empty Bytes
    let chunk = client.get_chunk(&issuer, &list_id, &999);
    assert_eq!(chunk.len(), 0);
}

// ---------------------------------------------------------------------------
// test_chunk_boundary_indices
// ---------------------------------------------------------------------------
#[test]
fn test_chunk_boundary_indices() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    // Create a list large enough to span multiple chunks
    // CHUNK_SIZE = 4096 bytes = 32768 bits
    create_test_list(&client, &issuer, &list_id, 65536);

    // First bit of first chunk
    client.set_status(&issuer, &list_id, &0, &true);
    assert!(client.is_revoked(&issuer, &list_id, &0));

    // Last bit of first chunk (bit 32767 = last bit of byte 4095)
    client.set_status(&issuer, &list_id, &32767, &true);
    assert!(client.is_revoked(&issuer, &list_id, &32767));

    // First bit of second chunk (bit 32768)
    client.set_status(&issuer, &list_id, &32768, &true);
    assert!(client.is_revoked(&issuer, &list_id, &32768));
}

// ---------------------------------------------------------------------------
// test_idempotent_re_set
// ---------------------------------------------------------------------------
#[test]
fn test_idempotent_re_set() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer, &list_id, 128);

    // Set the same bit multiple times
    client.set_status(&issuer, &list_id, &7, &true);
    assert!(client.is_revoked(&issuer, &list_id, &7));

    client.set_status(&issuer, &list_id, &7, &true); // idempotent re-set
    assert!(client.is_revoked(&issuer, &list_id, &7));

    client.set_status(&issuer, &list_id, &7, &false);
    assert!(!client.is_revoked(&issuer, &list_id, &7));

    client.set_status(&issuer, &list_id, &7, &false); // idempotent re-clear
    assert!(!client.is_revoked(&issuer, &list_id, &7));
}

// ---------------------------------------------------------------------------
// test_multiple_issuers_independent_lists
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_issuers_independent_lists() {
    let (e, client) = setup();
    let issuer_a = Address::generate(&e);
    let issuer_b = Address::generate(&e);
    let list_id = Symbol::new(&e, "main");

    create_test_list(&client, &issuer_a, &list_id, 128);
    create_test_list(&client, &issuer_b, &list_id, 128);

    // Revoke index 5 on issuer_a's list
    client.set_status(&issuer_a, &list_id, &5, &true);
    assert!(client.is_revoked(&issuer_a, &list_id, &5));
    assert!(!client.is_revoked(&issuer_b, &list_id, &5));

    // Revoke index 5 on issuer_b's list
    client.set_status(&issuer_b, &list_id, &5, &true);
    assert!(client.is_revoked(&issuer_a, &list_id, &5));
    assert!(client.is_revoked(&issuer_b, &list_id, &5));
}

// ---------------------------------------------------------------------------
// test_multiple_lists_per_issuer
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_lists_per_issuer() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_a = Symbol::new(&e, "list_a");
    let list_b = Symbol::new(&e, "list_b");

    create_test_list(&client, &issuer, &list_a, 64);
    create_test_list(&client, &issuer, &list_b, 64);

    client.set_status(&issuer, &list_a, &10, &true);
    assert!(client.is_revoked(&issuer, &list_a, &10));
    assert!(!client.is_revoked(&issuer, &list_b, &10));
}

// ---------------------------------------------------------------------------
// test_max_size_boundary_accepted
// ---------------------------------------------------------------------------
#[test]
fn test_max_size_boundary_accepted() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "max");

    // Exactly the maximum should be accepted
    client.create_list(&issuer, &list_id, &storage::MAX_LIST_SIZE);
    assert!(client.list_exists(&issuer, &list_id));
}

// ---------------------------------------------------------------------------
// test_single_bit_minimal_list
// ---------------------------------------------------------------------------
#[test]
fn test_single_bit_minimal_list() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "tiny");

    create_test_list(&client, &issuer, &list_id, 1);

    assert!(!client.is_revoked(&issuer, &list_id, &0));
    client.set_status(&issuer, &list_id, &0, &true);
    assert!(client.is_revoked(&issuer, &list_id, &0));
}

// ---------------------------------------------------------------------------
// test_get_list_metadata_nonexistent
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_get_list_metadata_nonexistent() {
    let (e, client) = setup();
    let issuer = Address::generate(&e);
    let list_id = Symbol::new(&e, "nonexistent");

    client.get_list_metadata(&issuer, &list_id); // must panic: ListNotFound
}
