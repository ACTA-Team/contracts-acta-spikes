#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, Bytes, Env, Symbol, TryFromVal,
};

use crate::contract::{VcSchemaRegistryContract, VcSchemaRegistryContractClient};

/// Returns true if the most recent contract call published an event whose
/// first topic is `topic`. `e.events().all()` only reflects the latest
/// top-level invocation, not a cumulative log across calls.
fn last_call_emitted(e: &Env, topic: &str) -> bool {
    let expected = Symbol::new(e, topic);
    e.events().all().iter().any(|(_, topics, _)| {
        topics
            .get(0)
            .and_then(|t| Symbol::try_from_val(e, &t).ok())
            .map(|s| s == expected)
            .unwrap_or(false)
    })
}

fn setup() -> (Env, VcSchemaRegistryContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcSchemaRegistryContract, ());
    let client = VcSchemaRegistryContractClient::new(&e, &contract_id);
    (e, client)
}

// ---------------------------------------------------------------------------
// test_initialize_sets_admin
// ---------------------------------------------------------------------------
#[test]
fn test_initialize_sets_admin() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);

    assert_eq!(client.admin(), admin);
}

// ---------------------------------------------------------------------------
// test_initialize_only_once
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_initialize_only_once() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);
    client.initialize(&admin); // must panic with AlreadyInitialized
}

// ---------------------------------------------------------------------------
// test_register_schema_happy_path
//   — Caller is the declared author; record is stored and queryable.
// ---------------------------------------------------------------------------
#[test]
fn test_register_schema_happy_path() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    assert!(client.schema_exists(&id));

    let record = client.get_schema(&id);
    assert_eq!(record.author, author);
    assert_eq!(record.uri, uri);
    assert!(!record.deprecated);
}

// ---------------------------------------------------------------------------
// test_register_schema_duplicate_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_register_schema_duplicate_rejected() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);
    client.register_schema(&id, &author, &uri); // must panic with SchemaAlreadyExists
}

// ---------------------------------------------------------------------------
// test_register_schema_uri_too_long_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_register_schema_uri_too_long_rejected() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let long_uri = Bytes::from_slice(&e, &[b'x'; 257]);

    client.initialize(&admin);
    client.register_schema(&id, &author, &long_uri); // must panic with InvalidUri
}

// ---------------------------------------------------------------------------
// test_register_schema_uri_boundary_ok
//   — A 256-byte uri (the max) must be accepted.
// ---------------------------------------------------------------------------
#[test]
fn test_register_schema_uri_boundary_ok() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let max_uri = Bytes::from_slice(&e, &[b'x'; 256]);

    client.initialize(&admin);
    client.register_schema(&id, &author, &max_uri);

    assert!(client.schema_exists(&id));
}

// ---------------------------------------------------------------------------
// test_register_schema_requires_author_auth
//   — A caller other than the declared author must be rejected.
// ---------------------------------------------------------------------------
#[test]
fn test_register_schema_requires_author_auth() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);

    // Clear mocks so no valid auth is provided for register_schema.
    e.mock_auths(&[]);
    let result = client.try_register_schema(&id, &author, &uri);

    assert!(result.is_err(), "register_schema without author auth must fail");
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_by_admin
// ---------------------------------------------------------------------------
#[test]
fn test_deprecate_schema_by_admin() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    client.deprecate_schema(&id, &admin);

    let record = client.get_schema(&id);
    assert!(record.deprecated);
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_by_author
// ---------------------------------------------------------------------------
#[test]
fn test_deprecate_schema_by_author() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    client.deprecate_schema(&id, &author);

    let record = client.get_schema(&id);
    assert!(record.deprecated);
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_non_admin_non_author_fails
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_deprecate_schema_non_admin_non_author_fails() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let stranger = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    client.deprecate_schema(&id, &stranger); // must panic with NotAuthorized
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_not_found
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_deprecate_schema_not_found() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"missing");

    client.initialize(&admin);
    client.deprecate_schema(&id, &admin); // must panic with SchemaNotFound
}

// ---------------------------------------------------------------------------
// test_update_schema_uri_by_author
// ---------------------------------------------------------------------------
#[test]
fn test_update_schema_uri_by_author() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri_v1 = Bytes::from_slice(&e, b"https://example.com/v1.json");
    let uri_v2 = Bytes::from_slice(&e, b"https://example.com/v2.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri_v1);

    client.update_schema_uri(&id, &author, &uri_v2);

    let record = client.get_schema(&id);
    assert_eq!(record.uri, uri_v2);
}

// ---------------------------------------------------------------------------
// test_update_schema_uri_non_author_fails
//   — Even the admin cannot update the uri; only the author may.
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_update_schema_uri_non_author_fails() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/v1.json");
    let new_uri = Bytes::from_slice(&e, b"https://example.com/v2.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    client.update_schema_uri(&id, &admin, &new_uri); // must panic with NotAuthorized
}

// ---------------------------------------------------------------------------
// test_update_schema_uri_too_long_rejected
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_update_schema_uri_too_long_rejected() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/v1.json");
    let long_uri = Bytes::from_slice(&e, &[b'x'; 257]);

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    client.update_schema_uri(&id, &author, &long_uri); // must panic with InvalidUri
}

// ---------------------------------------------------------------------------
// test_get_schema_not_found_panics
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_get_schema_not_found_panics() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"missing");

    client.initialize(&admin);
    client.get_schema(&id); // must panic with SchemaNotFound
}

// ---------------------------------------------------------------------------
// test_schema_exists_reflects_lifecycle
//   — schema_exists is true once registered, and stays true after
//     deprecation (deprecated schemas remain queryable, not deleted).
// ---------------------------------------------------------------------------
#[test]
fn test_schema_exists_reflects_lifecycle() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");
    let unknown = Bytes::from_slice(&e, b"unknown");

    client.initialize(&admin);
    assert!(!client.schema_exists(&id));

    client.register_schema(&id, &author, &uri);
    assert!(client.schema_exists(&id));

    client.deprecate_schema(&id, &admin);
    assert!(client.schema_exists(&id));
    assert!(client.get_schema(&id).deprecated);

    assert!(!client.schema_exists(&unknown));
}

// ---------------------------------------------------------------------------
// test_admin_not_initialized_panics
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_admin_not_initialized_panics() {
    let (_e, client) = setup();
    client.admin(); // must panic with NotInitialized
}

// ---------------------------------------------------------------------------
// test_version_returns_value
// ---------------------------------------------------------------------------
#[test]
fn test_version_returns_value() {
    let (_e, client) = setup();
    let version = client.version();
    assert!(!version.is_empty(), "version() must return a non-empty string");
}

// ---------------------------------------------------------------------------
// Event assertions
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_emits_event() {
    let (e, client) = setup();
    let admin = Address::generate(&e);

    client.initialize(&admin);

    assert!(last_call_emitted(&e, "initialized"), "initialize() must emit an event");
}

#[test]
fn test_register_schema_emits_event() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);

    client.register_schema(&id, &author, &uri);

    assert!(
        last_call_emitted(&e, "schema_registered"),
        "register_schema() must emit an event"
    );
}

#[test]
fn test_deprecate_schema_emits_event() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/schema-1.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    client.deprecate_schema(&id, &admin);

    assert!(
        last_call_emitted(&e, "schema_deprecated"),
        "deprecate_schema() must emit an event"
    );
}

#[test]
fn test_update_schema_uri_emits_event() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    let author = Address::generate(&e);
    let id = Bytes::from_slice(&e, b"schema-1");
    let uri = Bytes::from_slice(&e, b"https://example.com/v1.json");
    let new_uri = Bytes::from_slice(&e, b"https://example.com/v2.json");

    client.initialize(&admin);
    client.register_schema(&id, &author, &uri);

    client.update_schema_uri(&id, &author, &new_uri);

    assert!(
        last_call_emitted(&e, "schema_uri_updated"),
        "update_schema_uri() must emit an event"
    );
}
