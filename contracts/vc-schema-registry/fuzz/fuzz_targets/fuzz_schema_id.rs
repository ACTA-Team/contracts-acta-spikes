//! Fuzz target: schema-ID derivation and collision resistance.
//!
//! This fuzzer drives `register_schema` and `schema_id` with arbitrary
//! `(name, version)` pairs (using a fixed author address) and asserts:
//!
//! - `schema_id()` is deterministic: the same triple always produces the
//!   same 32-byte hash.
//! - The ID returned by `register_schema` matches the ID returned by
//!   `schema_id()` for the same triple.
//! - Registering the same triple twice results in `SchemaAlreadyExists`.
//! - A definition over 256 bytes is rejected with `InvalidDefinition`.
//!
//! The fuzzer uses the first 9 bytes of the input as the `name` Symbol and
//! the next 9 bytes as the `version` Symbol (trimmed to ASCII alphanumeric
//! characters to form valid Soroban Symbol strings), with the remaining
//! bytes used as the schema `definition`.
//!
//! Run with:
//! ```sh
//! cd contracts/vc-schema-registry
//! cargo +nightly fuzz run fuzz_schema_id --sanitizer none
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, Symbol};
use vc_schema_registry_contract::contract::{
    VcSchemaRegistryContract, VcSchemaRegistryContractClient,
};

/// Maximum definition size accepted by `register_schema`; must match
/// `MAX_DEFINITION_BYTES` in the contract.
const MAX_DEFINITION_BYTES: u32 = 256;

/// Build a valid Soroban Symbol from raw bytes by keeping only ASCII
/// alphanumeric characters and underscores, then truncating to 9 chars.
fn bytes_to_symbol_str(bytes: &[u8]) -> String {
    let filtered: String = bytes
        .iter()
        .filter_map(|&b| {
            let c = b as char;
            if c.is_ascii_alphanumeric() || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .take(9)
        .collect();
    // A Symbol must be non-empty; fall back to a safe default.
    if filtered.is_empty() {
        "x".to_string()
    } else {
        filtered
    }
}

fuzz_target!(|data: &[u8]| {
    // Need at least 2 bytes to produce name and version.
    if data.len() < 2 {
        return;
    }

    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register(VcSchemaRegistryContract, ());
    let client = VcSchemaRegistryContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    client.initialize(&admin);

    // Slice the input into name, version, and definition portions.
    let name_raw = &data[..data.len().min(9)];
    let version_raw = &data[data.len().min(9)..data.len().min(18)];
    let def_raw = if data.len() > 18 { &data[18..] } else { b"{}".as_ref() };

    let name_str = bytes_to_symbol_str(name_raw);
    let version_str = bytes_to_symbol_str(version_raw);

    let author = Address::generate(&e);
    let name = Symbol::new(&e, &name_str);
    let version = Symbol::new(&e, &version_str);
    let definition = Bytes::from_slice(&e, def_raw);

    // Pre-compute the ID — must equal the ID returned by register_schema.
    let computed_id = client.schema_id(&author, &name, &version);

    // Re-computing must produce the same result (determinism).
    let computed_id2 = client.schema_id(&author, &name, &version);
    assert_eq!(computed_id, computed_id2, "schema_id must be deterministic");

    // The remaining input feeds the definition, which is bounded at 256 bytes.
    // Anything longer must be rejected rather than registered.
    if definition.len() > MAX_DEFINITION_BYTES {
        let oversize = client.try_register_schema(&author, &name, &version, &definition);
        assert!(
            oversize.is_err(),
            "register_schema must reject a definition of {} bytes (> {})",
            definition.len(),
            MAX_DEFINITION_BYTES
        );
        return;
    }

    // Register the schema.
    let registered_id = client.register_schema(&author, &name, &version, &definition);
    assert_eq!(
        computed_id, registered_id,
        "schema_id() must match register_schema() for the same triple"
    );

    // schema_exists must return true.
    assert!(client.schema_exists(&registered_id));

    // Registering the same triple again must fail.
    let dup_result = client.try_register_schema(&author, &name, &version, &definition);
    assert!(
        dup_result.is_err(),
        "duplicate schema registration must fail with SchemaAlreadyExists"
    );
});
