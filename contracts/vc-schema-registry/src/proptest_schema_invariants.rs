//! Property-based tests for the VcSchemaRegistry invariants.
//!
//! Properties verified:
//!
//! 1. **Schema-ID collision resistance.** Distinct `(author, name, version)` triples
//!    must produce distinct schema IDs.
//! 2. **Determinism.** The same triple always produces the same ID regardless of
//!    how many times `schema_id()` is called.
//! 3. **Deprecation monotonicity.** A schema can never transition from
//!    `deprecated = true` back to `deprecated = false`. Once deprecated, it
//!    stays deprecated.
//! 4. **`schema_exists` reflects registration.** After `register_schema` the ID
//!    returned by `schema_id()` must be found by `schema_exists`.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, Symbol};

use crate::contract::{VcSchemaRegistryContract, VcSchemaRegistryContractClient};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate short, valid Symbol strings (up to 9 ASCII alphanum + '_' chars —
/// the Soroban Symbol length limit is 9 for small symbols).
fn symbol_str() -> impl Strategy<Value = std::string::String> {
    prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{0,8}").unwrap()
}

fn make_symbol(e: &Env, s: &str) -> Symbol {
    Symbol::new(e, s)
}

fn make_bytes(e: &Env, data: &[u8]) -> Bytes {
    Bytes::from_slice(e, data)
}

fn setup() -> (Env, VcSchemaRegistryContractClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcSchemaRegistryContract, ());
    let client = VcSchemaRegistryContractClient::new(&e, &contract_id);
    (e, client)
}

// ---------------------------------------------------------------------------
// Proptest: schema_id() is deterministic
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_schema_id_is_deterministic(
        name_str in symbol_str(),
        version_str in symbol_str(),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let author = Address::generate(&e);
        let name = make_symbol(&e, &name_str);
        let version = make_symbol(&e, &version_str);

        let id1 = client.schema_id(&author, &name, &version);
        let id2 = client.schema_id(&author, &name, &version);

        prop_assert_eq!(id1, id2, "schema_id must be deterministic for the same inputs");
    }
}

// ---------------------------------------------------------------------------
// Proptest: distinct triples produce distinct IDs
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_distinct_triples_produce_distinct_ids(
        name_a in symbol_str(),
        name_b in symbol_str(),
        version_str in symbol_str(),
    ) {
        // Only test when name strings differ to guarantee different triples
        prop_assume!(name_a != name_b);

        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let author = Address::generate(&e);
        let version = make_symbol(&e, &version_str);

        let id_a = client.schema_id(&author, &make_symbol(&e, &name_a), &version);
        let id_b = client.schema_id(&author, &make_symbol(&e, &name_b), &version);

        prop_assert_ne!(id_a, id_b, "different schema names must produce different IDs");
    }
}

// ---------------------------------------------------------------------------
// Proptest: distinct versions produce distinct IDs
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_distinct_versions_produce_distinct_ids(
        name_str in symbol_str(),
        version_a in symbol_str(),
        version_b in symbol_str(),
    ) {
        prop_assume!(version_a != version_b);

        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let author = Address::generate(&e);
        let name = make_symbol(&e, &name_str);

        let id_a = client.schema_id(&author, &name, &make_symbol(&e, &version_a));
        let id_b = client.schema_id(&author, &name, &make_symbol(&e, &version_b));

        prop_assert_ne!(id_a, id_b, "different schema versions must produce different IDs");
    }
}

// ---------------------------------------------------------------------------
// Proptest: schema_id matches the ID returned by register_schema
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_schema_id_matches_registered_id(
        name_str in symbol_str(),
        version_str in symbol_str(),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let author = Address::generate(&e);
        let name = make_symbol(&e, &name_str);
        let version = make_symbol(&e, &version_str);
        let def = make_bytes(&e, b"{}");

        let computed_id = client.schema_id(&author, &name, &version);
        let registered_id = client.register_schema(&author, &name, &version, &def);

        prop_assert_eq!(computed_id, registered_id,
            "schema_id() must match the ID returned by register_schema()");
    }
}

// ---------------------------------------------------------------------------
// Proptest: after register_schema, schema_exists returns true
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_schema_exists_after_registration(
        name_str in symbol_str(),
        version_str in symbol_str(),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let author = Address::generate(&e);
        let name = make_symbol(&e, &name_str);
        let version = make_symbol(&e, &version_str);
        let def = make_bytes(&e, b"{}");

        let schema_id = client.register_schema(&author, &name, &version, &def);

        prop_assert!(
            client.schema_exists(&schema_id),
            "schema_exists must return true after registration"
        );
    }
}

// ---------------------------------------------------------------------------
// Proptest: deprecation monotonicity — deprecated never goes back to false
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_deprecation_is_monotone(
        name_str in symbol_str(),
        version_str in symbol_str(),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let author = Address::generate(&e);
        let name = make_symbol(&e, &name_str);
        let version = make_symbol(&e, &version_str);
        let def = make_bytes(&e, b"{}");

        let schema_id = client.register_schema(&author, &name, &version, &def);

        // Before deprecation: deprecated = false
        let before = client.get_schema(&schema_id);
        prop_assert!(!before.deprecated);

        // Deprecate
        client.deprecate_schema(&schema_id, &admin);
        let after = client.get_schema(&schema_id);
        prop_assert!(after.deprecated, "deprecated flag must be true after deprecation");

        // schema_exists still returns true (non-destructive)
        prop_assert!(client.schema_exists(&schema_id));

        // Second deprecation attempt must fail (AlreadyDeprecated)
        let result = client.try_deprecate_schema(&schema_id, &admin);
        prop_assert!(
            result.is_err(),
            "deprecating an already-deprecated schema must fail"
        );
    }
}

// ---------------------------------------------------------------------------
// Proptest: two different authors always get different IDs for the same name/version
// ---------------------------------------------------------------------------
proptest! {
    #[test]
    fn prop_different_authors_get_different_ids(
        name_str in symbol_str(),
        version_str in symbol_str(),
    ) {
        let (e, client) = setup();
        let admin = Address::generate(&e);
        client.initialize(&admin);

        let author_a = Address::generate(&e);
        let author_b = Address::generate(&e);
        // In practice two generated addresses are always distinct, but add
        // an assumption guard to be safe.
        prop_assume!(author_a != author_b);

        let name = make_symbol(&e, &name_str);
        let version = make_symbol(&e, &version_str);

        let id_a = client.schema_id(&author_a, &name, &version);
        let id_b = client.schema_id(&author_b, &name, &version);

        prop_assert_ne!(id_a, id_b, "different authors must produce different schema IDs");
    }
}
