#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env, Symbol};

use crate::contract::{VcSchemaRegistryContract, VcSchemaRegistryContractClient};

fn setup() -> (Env, VcSchemaRegistryContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcSchemaRegistryContract, ());
    let client = VcSchemaRegistryContractClient::new(&e, &contract_id);
    (e, client)
}

fn sample_schema(e: &Env) -> (Address, Symbol, Symbol, Bytes) {
    let author = Address::generate(e);
    let name = Symbol::new(e, "IdentitySchema");
    let version = Symbol::new(e, "v1");
    let definition = Bytes::from_slice(e, b"{\"type\":\"object\"}");
    (author, name, version, definition)
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
//   — register_schema returns a non-zero ID, stores the record with
//     deprecated=false, and schema_exists returns true.
// ---------------------------------------------------------------------------
#[test]
fn test_register_schema_happy_path() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    let schema_id = client.register_schema(&author, &name, &version, &definition);

    assert!(client.schema_exists(&schema_id));

    let record = client.get_schema(&schema_id);
    assert_eq!(record.author, author);
    assert_eq!(record.name, name);
    assert_eq!(record.version, version);
    assert_eq!(record.definition, definition);
    assert!(!record.deprecated);
}

// ---------------------------------------------------------------------------
// test_register_schema_id_is_deterministic
//   — Calling schema_id() with the same inputs returns the same value as
//     the ID returned by register_schema.
// ---------------------------------------------------------------------------
#[test]
fn test_register_schema_id_is_deterministic() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    let registered_id = client.register_schema(&author, &name, &version, &definition);
    let computed_id = client.schema_id(&author, &name, &version);

    assert_eq!(registered_id, computed_id);
}

// ---------------------------------------------------------------------------
// test_register_schema_duplicate_rejected
//   — Registering the same (author, name, version) triple twice must panic.
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_register_schema_duplicate_rejected() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    client.register_schema(&author, &name, &version, &definition);
    client.register_schema(&author, &name, &version, &definition); // must panic
}

// ---------------------------------------------------------------------------
// test_different_versions_are_independent
//   — Same author and name but different versions produce distinct IDs and
//     can both be registered.
// ---------------------------------------------------------------------------
#[test]
fn test_different_versions_are_independent() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let author = Address::generate(&e);
    let name = Symbol::new(&e, "MySchema");
    let def = Bytes::from_slice(&e, b"{}");

    let id_v1 = client.register_schema(&author, &name, &Symbol::new(&e, "v1"), &def);
    let id_v2 = client.register_schema(&author, &name, &Symbol::new(&e, "v2"), &def);

    assert_ne!(id_v1, id_v2);
    assert!(client.schema_exists(&id_v1));
    assert!(client.schema_exists(&id_v2));
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_by_admin
//   — Admin can deprecate any schema; get_schema reflects deprecated=true.
// ---------------------------------------------------------------------------
#[test]
fn test_deprecate_schema_by_admin() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    let schema_id = client.register_schema(&author, &name, &version, &definition);

    client.deprecate_schema(&schema_id, &admin);

    let record = client.get_schema(&schema_id);
    assert!(record.deprecated);
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_by_author
//   — The schema author can deprecate their own schema.
// ---------------------------------------------------------------------------
#[test]
fn test_deprecate_schema_by_author() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    let schema_id = client.register_schema(&author, &name, &version, &definition);

    client.deprecate_schema(&schema_id, &author);

    let record = client.get_schema(&schema_id);
    assert!(record.deprecated);
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_unauthorized
//   — A random address that is neither admin nor author must be rejected.
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_deprecate_schema_unauthorized() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    let schema_id = client.register_schema(&author, &name, &version, &definition);

    let stranger = Address::generate(&e);
    client.deprecate_schema(&schema_id, &stranger); // must panic with Unauthorized
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_already_deprecated
//   — Calling deprecate_schema a second time must panic with AlreadyDeprecated.
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_deprecate_schema_already_deprecated() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    let schema_id = client.register_schema(&author, &name, &version, &definition);

    client.deprecate_schema(&schema_id, &admin);
    client.deprecate_schema(&schema_id, &admin); // must panic
}

// ---------------------------------------------------------------------------
// test_deprecate_schema_not_found
//   — Deprecating a non-existent schema must panic with SchemaNotFound.
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_deprecate_schema_not_found() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let fake_id: BytesN<32> = BytesN::from_array(&e, &[0u8; 32]);
    client.deprecate_schema(&fake_id, &admin); // must panic
}

// ---------------------------------------------------------------------------
// test_schema_exists_returns_false_for_unknown
// ---------------------------------------------------------------------------
#[test]
fn test_schema_exists_returns_false_for_unknown() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let unknown: BytesN<32> = BytesN::from_array(&e, &[1u8; 32]);
    assert!(!client.schema_exists(&unknown));
}

// ---------------------------------------------------------------------------
// test_get_schema_not_found_panics
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_get_schema_not_found_panics() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let fake: BytesN<32> = BytesN::from_array(&e, &[2u8; 32]);
    client.get_schema(&fake); // must panic
}

// ---------------------------------------------------------------------------
// test_calls_fail_before_initialize
//   — register_schema before initialize must panic with NotInitialized.
// ---------------------------------------------------------------------------
#[test]
#[should_panic]
fn test_register_before_initialize_panics() {
    let (e, client) = setup();
    let (author, name, version, definition) = sample_schema(&e);
    client.register_schema(&author, &name, &version, &definition); // must panic
}

// ---------------------------------------------------------------------------
// test_non_author_cannot_register_for_another
//   — When mocks are cleared, a caller that is not `author` must be rejected
//     by the host auth system.
// ---------------------------------------------------------------------------
#[test]
fn test_non_author_cannot_register_for_another() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let author = Address::generate(&e);
    let name = Symbol::new(&e, "Schema");
    let version = Symbol::new(&e, "v1");
    let definition = Bytes::from_slice(&e, b"{}");

    // Clear mocks so no valid auth is provided for the author address.
    e.mock_auths(&[]);
    let result = client.try_register_schema(&author, &name, &version, &definition);
    assert!(result.is_err(), "non-author call must fail auth");
}

// ---------------------------------------------------------------------------
// test_schema_id_differs_across_authors
//   — Two different authors registering the same name/version get different IDs.
// ---------------------------------------------------------------------------
#[test]
fn test_schema_id_differs_across_authors() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let name = Symbol::new(&e, "Schema");
    let version = Symbol::new(&e, "v1");

    let author_a = Address::generate(&e);
    let author_b = Address::generate(&e);

    let id_a = client.schema_id(&author_a, &name, &version);
    let id_b = client.schema_id(&author_b, &name, &version);

    assert_ne!(id_a, id_b);
}

// ---------------------------------------------------------------------------
// test_deprecation_preserves_record
//   — After deprecation get_schema still returns the full record.
// ---------------------------------------------------------------------------
#[test]
fn test_deprecation_preserves_record() {
    let (e, client) = setup();
    let admin = Address::generate(&e);
    client.initialize(&admin);

    let (author, name, version, definition) = sample_schema(&e);
    let schema_id = client.register_schema(&author, &name, &version, &definition);

    client.deprecate_schema(&schema_id, &admin);

    let record = client.get_schema(&schema_id);
    assert_eq!(record.author, author);
    assert_eq!(record.definition, definition);
    assert!(record.deprecated, "deprecated flag must be set");
    // schema_exists returns true even for deprecated schemas
    assert!(client.schema_exists(&schema_id));
}
