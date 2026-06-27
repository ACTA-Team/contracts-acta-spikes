# vc-revocation-registry

A Stellar smart contract that maintains an on-chain registry of revoked Verifiable Credentials.

## Overview

`vc-revocation-registry` is a standalone contract that separates credential revocation status from VC storage. It answers the question: _"Has this credential been revoked?"_ — without coupling that logic to `vc-vault`.

The contract provides admin-controlled revocation tracking, allowing issuers to mark credentials as no longer valid and later unrevoke them if needed.

## Storage layout

| Key                          | Storage type | Description                                    |
| ---------------------------- | ------------ | ---------------------------------------------- |
| `Admin`                      | Instance     | Contract admin address                         |
| `Revocation(Address, Bytes)` | Persistent   | `RevocationRecord` for each revoked credential |

### RevocationRecord

```rust
pub struct RevocationRecord {
    pub revoked_at: u64,
}
```

## Entry points

| Function                                | Auth  | Description                                            |
| --------------------------------------- | ----- | ------------------------------------------------------ |
| `initialize(admin)`                     | admin | One-time init; stores admin                            |
| `revoke(issuer, credential_id)`         | admin | Mark a credential as revoked                           |
| `unrevoke(issuer, credential_id)`       | admin | Remove a credential from the revocation registry       |
| `is_revoked(issuer, credential_id)`     | —     | Return `true` if credential is revoked                 |
| `get_revocation(issuer, credential_id)` | —     | Return full `RevocationRecord` (panics if not revoked) |
| `admin()`                               | —     | Return current admin address                           |
| `version()`                             | —     | Return crate version string                            |

### Revocation semantics

#### `revoke`

Marks a credential as revoked by recording it with the current ledger timestamp. After revocation:

- `is_revoked` returns `true`.
- `get_revocation` returns the `RevocationRecord` with the revocation timestamp.
- Queries for that credential should be rejected by the verifier.

Revoke fails with `CredentialAlreadyExists` if the credential is already revoked. To correct a revocation, use `unrevoke` first, then `revoke` again.

#### `unrevoke`

Removes a credential from the revocation registry entirely (hard delete). Call this when:

- A revocation was made in error and should be withdrawn.
- A previously revoked credential should no longer be considered revoked.

After unrevoke:

- `is_revoked` returns `false`.
- `get_revocation` panics with `CredentialNotFound`.
- The same credential can be revoked again with `revoke`.

**Note:** `unrevoke` is **not** the same as issuing a new credential; it removes the revocation status entirely. Use it sparingly — once a credential is unrevoked, verifiers will treat it as if it was never revoked.

### Credential ID validation

`credential_id` fields are bounded to a maximum of **256 bytes** each.
`revoke` and `unrevoke` will panic with `InvalidCredentialId` if the credential ID exceeds this limit.

## Error codes

| Code | Variant                   | Meaning                               |
| ---- | ------------------------- | ------------------------------------- |
| 1    | `AlreadyInitialized`      | `initialize` called more than once    |
| 2    | `CredentialNotFound`      | Credential not in revocation registry |
| 3    | `CredentialAlreadyExists` | Credential already revoked            |
| 4    | `NotInitialized`          | Contract not yet initialized          |
| 5    | `InvalidCredentialId`     | Credential ID exceeds max byte length |

## Events

| Event                 | Emitted by   | Fields                                  |
| --------------------- | ------------ | --------------------------------------- |
| `Initialized`         | `initialize` | `admin: Address`                        |
| `CredentialRevoked`   | `revoke`     | `issuer: Address, credential_id: Bytes` |
| `CredentialUnrevoked` | `unrevoke`   | `issuer: Address, credential_id: Bytes` |

## Build & test

```bash
# from repo root
cargo build -p vc-revocation-registry
cargo test -p vc-revocation-registry

# WASM
stellar contract build
```
