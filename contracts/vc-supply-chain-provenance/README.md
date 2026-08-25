# vc-supply-chain-provenance

`vc-supply-chain-provenance` is a Soroban contract that tracks supply-chain custody as an append-only chain, ends in a terminal sealed state, and rejects batches whose origin certificate has been revoked in `vc-revocation-registry`.

## Purpose

Registries in this repo answer point-in-time questions (is this issuer allowed, is this credential revoked). Supply-chain provenance is different: the answer is a **history**. A coffee lot, a batch of vaccines, or a shipment of minerals: what matters is who held it, in what order, and whether the origin certificate backing it is still valid.

## Storage layout

Each custody hop is stored under its own persistent key. The `Batch` record holds only a hop counter (`hops: u32`), not a growing `Vec<CustodyHop>`. Loading and rewriting the entire chain on every transfer would make cost grow without bound as history lengthens.

| Key | Storage type | Description |
| --- | --- | --- |
| `Admin` (registry-core) | Instance | Contract admin |
| `RevocationRegistry` | Instance | `vc-revocation-registry` contract address |
| `Batch(BytesN<32>)` | Persistent | Batch metadata and current state |
| `Hop(BytesN<32>, u32)` | Persistent | One `CustodyHop` per index (0-based) |
| `Certificate(BytesN<32>)` | Persistent | Attached `credential_id` for the batch |

### Batch

```rust
pub struct Batch {
    pub certifier: Address,
    pub custodian: Address,
    pub product: Symbol,
    pub origin: Symbol,
    pub state: BatchState,
    pub hops: u32,
    pub created_at: u64,
    pub sealed_at: u64, // 0 while InTransit
    pub metadata: Bytes,
}
```

## Public API

### Initialization

- `initialize(env, admin: Address, revocation_registry: Address)`
  - One-time setup. Stores admin and revocation registry address.
  - Second call panics with `CommonError::AlreadyInitialized` (code 1).

### Batch lifecycle

- `register_batch(env, certifier, batch_id, product, origin, metadata)`
  - `certifier.require_auth()`. Fails if `batch_id` already exists.
  - Certifier becomes the initial custodian. `metadata` bounded by `registry_core::DEFAULT_MAX_BYTES` (256).

- `attach_certificate(env, certifier, batch_id, credential_id)`
  - `certifier.require_auth()`. Rejects revoked credentials via live cross-contract check.
  - Replacement allowed only while `InTransit`.

- `transfer_custody(env, batch_id, from, to)`
  - `from.require_auth()`. Appends one hop and updates custodian.
  - Fails when sealed, when `from` is not custodian, when `to == from`, or when `hops >= MAX_HOPS` (100).

- `seal_batch(env, batch_id, custodian)`
  - `custodian.require_auth()`. Terminal: no further transfers or certificate changes.

### Queries

- `get_batch(env, batch_id) -> Batch`
- `hop_count(env, batch_id) -> u32`
- `get_custody_chain(env, batch_id, start, limit) -> Vec<CustodyHop>`
  - `limit` must be `<= MAX_CHAIN_PAGE` (50). Returns fewer entries near the end of the chain.
- `is_provenance_valid(env, batch_id) -> bool`
  - True only when sealed, certificate attached, and credential not revoked at call time.
- `version(env) -> Symbol`
  - Returns `"0_1_0"`.

## Error codes

Shared codes (`registry_core::CommonError`):

| Code | Variant | When |
| --- | --- | --- |
| 1 | AlreadyInitialized | Second `initialize` |
| 2 | NotFound | Unknown `batch_id` |
| 3 | AlreadyExists | Duplicate `register_batch` |
| 4 | NotInitialized | Call before `initialize` |
| 5 | Unauthorized | Caller is not the batch certifier |
| 6 | InvalidInput | Oversized `metadata` or `credential_id` |

Contract-specific codes:

| Code | Variant | When |
| --- | --- | --- |
| 10 | NotCustodian | Caller is not the current custodian |
| 11 | BatchSealed | Mutation on a sealed batch |
| 12 | SelfTransfer | `to == from` |
| 13 | HopLimitExceeded | `hops >= MAX_HOPS` |
| 14 | CertificateRevoked | Credential revoked in registry |
| 15 | NoCertificateAttached | Seal without certificate |
| 16 | LimitTooLarge | `limit > MAX_CHAIN_PAGE` |

## Events

| Event | Emitted by | Fields |
| --- | --- | --- |
| `BatchRegistered` | `register_batch` | `batch_id`, `certifier`, `product`, `origin` |
| `CertificateAttached` | `attach_certificate` | `batch_id`, `certifier`, `credential_id` |
| `CustodyTransferred` | `transfer_custody` | `batch_id`, `from`, `to`, `hop_index` |
| `BatchSealed` | `seal_batch` | `batch_id`, `custodian` |

## Testing

```bash
cargo test -p vc-supply-chain-provenance-contract
cd contracts/vc-supply-chain-provenance && cargo +nightly fuzz run fuzz_custody_chain --sanitizer none -- -max_total_time=60
```

Property tests cover custody invariants (custodian matches chain, hop immutability, non-decreasing timestamps, terminal sealing, monotonic hop count).
