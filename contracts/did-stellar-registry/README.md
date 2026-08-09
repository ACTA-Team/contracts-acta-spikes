# DID Stellar Registry

A Soroban smart contract implementing the `did:stellar` DID method. Controllers
register self-sovereign DID documents on Stellar; the contract handles
registration, update, deactivation, and resolution.

---

## Overview

Unlike the other registries in this workspace (`vc-issuer-registry`,
`vc-schema-registry`), **there is no admin key**. Each Stellar address is its
own authority: a controller registers and manages their own DID, and no
privileged account can interfere. This is self-sovereign identity by design.

---

## DID Method

**Method name:** `did:stellar`

**DID syntax:**

```
did:stellar:<base64url(sha256(xdr(controller_address)))>
```

where `xdr(controller_address)` is the canonical XDR encoding of the
controller's `ScVal::Address`.

### Derivation algorithm

```
preimage = to_xdr(controller_address)   // XDR-encoded ScVal::Address
did_id   = sha256(preimage)             // 32 bytes
did      = "did:stellar:" + base64url(did_id)
```

The 32-byte `did_id` is what the contract stores and returns. The
`did:stellar:` prefix and base64url encoding are a presentation concern for
off-chain tooling.

**Reproducing off-chain:** any XDR-aware SDK can compute the same `did_id`
without a network call:

```python
import hashlib, base64
from stellar_sdk import Keypair
from stellar_sdk.xdr import SCVal, SCAddress, SCValType

address = "GBXGQJWVLWOYHFLBCZQE47ZHY5AQBGCXUANWIY7RXRP3MYFKHZUZI7K"
# Serialize as ScVal::Address XDR, then SHA-256
did_id = hashlib.sha256(scval_address_xdr(address)).digest()
did    = "did:stellar:" + base64.urlsafe_b64encode(did_id).rstrip(b"=").decode()
```

The contract exposes a read-only `did_for(controller)` entry point that
performs this computation on-chain, mirroring `schema_id()` in
`vc-schema-registry`.

---

## Storage layout

| Key | Storage tier | Type | Description |
|-----|-------------|------|-------------|
| `Did(BytesN<32>)` | Persistent | `DidRecord` | Per-DID record, keyed by DID identifier |

### `DidRecord` fields

| Field | Type | Description |
|-------|------|-------------|
| `controller` | `Address` | Stellar address that owns this DID |
| `document` | `Bytes` | Raw DID document (max 1024 bytes) |
| `version` | `u32` | Starts at 1; incremented on every `update` |
| `deactivated` | `bool` | `true` after `deactivate`; record is never deleted |
| `created_at` | `u64` | Ledger timestamp at registration |
| `updated_at` | `u64` | Ledger timestamp of last mutation |

---

## Entry points

| Function | Auth | Description |
|----------|------|-------------|
| `register(controller, document) -> Bytes` | `controller` | Register a new DID; returns the 32-byte DID identifier |
| `update(controller, document)` | `controller` | Replace the DID document; bumps `version` |
| `deactivate(controller)` | `controller` | Mark the DID as inactive; record persists |
| `resolve(did) -> DidRecord` | none | Return the full record for a DID |
| `is_active(did) -> bool` | none | `true` iff the DID exists and is not deactivated |
| `did_for(controller) -> Bytes` | none | Compute the DID identifier without writing |
| `version() -> String` | none | Contract version from `Cargo.toml` |

---

## Authorization model

**Self-sovereign — no admin.** Every mutating entry point requires only the
controller's own signature via `controller.require_auth()`. There is no
`initialize` call, no admin address, and no privileged escalation path.

This is a deliberate departure from `vc-issuer-registry` and
`vc-schema-registry`, which are admin-gated. The rationale: a DID is an
identifier that belongs to its subject; no third party should be able to
register, update, or deactivate it.

---

## Document constraints

DID documents are stored inline and capped at **1024 bytes**. A minimal W3C
DID document with `id`, `verificationMethod`, and `authentication` fields fits
in approximately 400 bytes; 1024 leaves room for one or two service endpoints.

If a document exceeds 1024 bytes, `register` and `update` panic with
`DocumentTooLarge` (code 5). A future version may support a hash+off-chain-URI
pattern to lift this limit.

---

## Error codes

| Code | Name | When |
|------|------|------|
| 1 | `DidAlreadyExists` | `register` called for a controller that already has a DID |
| 2 | `DidNotFound` | `resolve` or `update` or `deactivate` on an unknown DID |
| 3 | `DidDeactivated` | `update` or `deactivate` on an already-deactivated DID |
| 4 | `Unauthorized` | Internal controller mismatch (defence-in-depth) |
| 5 | `DocumentTooLarge` | Document exceeds 1024 bytes |

---

## Events

| Event | Fields | Emitted by |
|-------|--------|------------|
| `DidRegistered` | `did: BytesN<32>`, `controller: Address` | `register` |
| `DidUpdated` | `did: BytesN<32>`, `controller: Address`, `version: u32` | `update` |
| `DidDeactivated` | `did: BytesN<32>`, `controller: Address` | `deactivate` |

---

## Deactivate, never delete

Credentials signed by a key listed in an old DID document must remain
verifiable. For this reason:

- `deactivate` sets `deactivated = true` and keeps the record on-chain.
- A deactivated DID **cannot** be updated or re-registered.
- `resolve` returns the full record (including `deactivated = true`).
- `is_active` returns `false` for deactivated DIDs.

Verifiers should check `is_active` before trusting a credential's key material.

---

## Build and test

```bash
# Run tests
cargo test -p did-stellar-registry

# Build optimized WASM
./scripts/build.sh

# Deploy to testnet
./scripts/deploy.sh testnet <your-account>
```
