# vc-verifier

A thin orchestrator contract that answers the one question every relying party actually asks:

> **Is this credential currently valid?**

Instead of three separate RPC calls to three contracts with client-side combination logic, a single `verify` call returns a structured breakdown.

## Interface

### `initialize`

```rust
pub fn initialize(
    e: Env,
    admin: Address,
    issuer_registry: Address,
    schema_registry: Address,
    revocation_registry: Address,
)
```

Admin-only, one time. Wires the verifier to the three registries.

### `verify`

```rust
pub fn verify(
    e: Env,
    issuer: Address,
    schema_id: BytesN<32>,
    credential_id: Bytes,
) -> VerificationResult
```

Pure read — no storage writes. Returns a full breakdown:

```rust
pub struct VerificationResult {
    pub valid: bool,            // all four conditions pass
    pub issuer_allowed: bool,   // issuer is on the allowlist
    pub schema_exists: bool,    // schema ID found in registry
    pub schema_deprecated: bool,// schema has been deprecated
    pub revoked: bool,          // credential has been revoked
}
```

**Policy:** `valid = issuer_allowed && schema_exists && !schema_deprecated && !revoked`

A deprecated schema is treated as fatal for verification. Issuers must migrate to an active schema for new credentials to be considered valid.

### Registry address updates (admin-only)

```rust
pub fn set_issuer_registry(e: Env, new_address: Address)
pub fn set_schema_registry(e: Env, new_address: Address)
pub fn set_revocation_registry(e: Env, new_address: Address)
```

Each emits an event on change so observers can track registry redeployments.

## End-to-end example

```
1. Register issuer
   issuer_registry::add_issuer(admin, issuer, None, None, None)

2. Register schema
   schema_id = schema_registry::register_schema(
       issuer, "IdentitySchema", "v1", b"{...json schema...}"
   )

3. Verify (should be valid)
   result = verifier::verify(issuer, schema_id, credential_id)
   // result.valid == true

4. Revoke credential
   revocation_registry::revoke(issuer, credential_id)

5. Verify again (now revoked)
   result = verifier::verify(issuer, schema_id, credential_id)
   // result.valid == false, result.revoked == true
```

## Design notes

- Uses `#[contractclient]` inline trait bindings — no dependency on registry crates at runtime, avoiding testutils feature bleed.
- `verify` calls the non-panicking predicates (`is_issuer_allowed`, `schema_exists`, `is_revoked`) so a missing record returns `valid: false` instead of aborting.
- Registry addresses are updatable by admin to handle redeployments.
- All registry dependencies are dev-only (testutils) for tests; production binary has zero cross-crate deps.
