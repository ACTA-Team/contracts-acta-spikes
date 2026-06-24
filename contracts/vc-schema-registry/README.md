# vc-schema-registry

A Stellar smart contract that provides an on-chain registry for Verifiable Credential schema definitions.

## Overview

`vc-schema-registry` lets schema authors publish versioned JSON Schema (or other byte-encoded) definitions on-chain and tie them permanently to their address. Downstream contracts and clients — such as `vc-issuer-registry` — can reference a schema ID to prove that a credential type has a published, auditable definition.

Each schema is identified by a deterministic 32-byte ID derived from the author address, schema name, and version. Schemas can be deprecated by the admin or the original author without being deleted, preserving the historical record while signalling that new credentials should not reference that version.

## Storage layout

| Key                   | Storage type | Description                                 |
| --------------------- | ------------ | ------------------------------------------- |
| `Admin`               | Instance     | Contract admin address                      |
| `Schema(BytesN<32>)`  | Persistent   | `SchemaRecord` keyed by schema ID           |

### SchemaRecord

```rust
pub struct SchemaRecord {
    pub author: Address,
    pub name: Symbol,
    pub version: Symbol,
    pub definition: Bytes,
    pub deprecated: bool,
}
```

## Entry points

| Function                                                  | Auth            | Description                                                                    |
| --------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------ |
| `initialize(admin)`                                       | admin           | One-time init; stores admin address                                            |
| `register_schema(author, name, version, definition)`      | author          | Publish a new schema; returns the computed `schema_id`                         |
| `deprecate_schema(schema_id, caller)`                     | admin or author | Mark a schema deprecated (non-destructive)                                     |
| `get_schema(schema_id)`                                   | —               | Return full `SchemaRecord`; panics with `SchemaNotFound` if missing            |
| `schema_exists(schema_id)`                                | —               | Return `true` if a schema with that ID exists (includes deprecated)            |
| `schema_id(author, name, version)`                        | —               | Compute and return a schema ID without writing to storage                      |
| `admin()`                                                 | —               | Return current admin address                                                   |
| `version()`                                               | —               | Return crate version string                                                    |

### Auth model

`register_schema` requires the `author` address to authorize the call. This prevents one party from registering schemas under another party's address.

`deprecate_schema` accepts a `caller` address that must authorize and be either the contract admin **or** the author of that specific schema. Passing any other address panics with `Unauthorized`.

## Schema ID derivation

A schema ID is the SHA-256 hash of the XDR-encoded author address, name, and version concatenated in order:

```text
schema_id = sha256( xdr(author) || xdr(name) || xdr(version) )
```

XDR encoding is used for each component so the preimage matches the canonical `ScVal` serialization. This means the same ID can be reproduced off-chain using any XDR-aware Stellar SDK by serializing the same three values and hashing the result.

The `schema_id` entry point exposes this computation as a read-only call, making it easy for frontends and tooling to predict the ID before submitting a `register_schema` transaction.

## Versioning & deprecation

Every `(author, name, version)` triple maps to exactly one schema ID. To publish a revised schema, the author registers a new version (e.g. `v2`) — this creates a new independent record without altering or deleting the original.

Deprecation is always **non-destructive**:

- `get_schema` continues to return the full record with `deprecated = true`.
- `schema_exists` returns `true` for deprecated schemas.
- No on-chain data is deleted.

Consumers are responsible for checking the `deprecated` flag and refusing to accept new credentials that reference a deprecated schema ID.

## Error codes

| Code | Variant             | Meaning                                              |
| ---- | ------------------- | ---------------------------------------------------- |
| 1    | `AlreadyInitialized`  | `initialize` called more than once                 |
| 2    | `SchemaNotFound`      | Schema ID not in registry                          |
| 3    | `SchemaAlreadyExists` | Same `(author, name, version)` already registered  |
| 4    | `NotInitialized`      | Contract not yet initialized                       |
| 5    | `AlreadyDeprecated`   | Schema is already deprecated                       |
| 6    | `Unauthorized`        | Caller is neither admin nor the schema author      |

## Events

| Event              | Emitted by          | Fields                                    |
| ------------------ | ------------------- | ----------------------------------------- |
| `Initialized`      | `initialize`        | `admin: Address`                          |
| `SchemaRegistered` | `register_schema`   | `schema_id: BytesN<32>, author: Address`  |
| `SchemaDeprecated` | `deprecate_schema`  | `schema_id: BytesN<32>`                   |

## Build & test

```bash
# from repo root
cargo build -p vc-schema-registry-contract
cargo test -p vc-schema-registry-contract

# WASM
stellar contract build
```
