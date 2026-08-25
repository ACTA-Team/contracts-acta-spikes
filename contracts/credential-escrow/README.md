# credential-escrow

`credential-escrow` is a Soroban contract that custodies SEP-41 tokens and settles exactly once: release to the beneficiary when a live credential verifies, or refund to the depositor after the deadline.

## Purpose

Grant programs, employers, and insurers often need funds locked up front and released only against a verified credential. This contract holds tokens in escrow, calls `vc-verifier::verify` on claim, and returns funds to the depositor if the deadline passes without a valid claim.

## Storage layout

| Key | Storage type | Description |
| --- | --- | --- |
| `Admin` (registry-core) | Instance | Contract admin |
| `Verifier` | Instance | `vc-verifier` contract address |
| `NextEscrowId` | Instance | Strictly increasing escrow counter |
| `Escrow(u64)` | Persistent | Escrow record and settlement state |

## Settlement rules

- **`create_escrow`**: `depositor.require_auth()`. `amount > 0`, `deadline > now`, `beneficiary != depositor`. Transfers tokens **into** the contract, then records the escrow.
- **`claim`**: `beneficiary.require_auth()`. Requires `Funded` and `now < deadline`. Calls `verifier.verify(required_issuer, schema_id, credential_id)`; proceeds only when `valid == true`. Writes `Claimed`, then transfers to beneficiary.
- **`refund`**: `depositor.require_auth()`. Requires `Funded` and `now >= deadline`. Writes `Refunded`, then transfers to depositor.

There is no instant at which both `claim` and `refund` are legal.

## State-before-transfer ordering

Outbound settlement (`claim` / `refund`) writes the terminal state (`Claimed` or `Refunded`) **before** the token transfer. A re-entrant call during transfer sees a settled escrow and cannot pay out again.

Inbound funding (`create_escrow`) transfers tokens **before** persisting the escrow record. If the inbound transfer fails, no `Funded` record exists without backing tokens.

## Public API

### Initialization

- `initialize(env, admin: Address, verifier: Address)`
  - One-time setup. Stores admin and verifier address.
  - Second call panics with `CommonError::AlreadyInitialized` (code 1).

### Escrow lifecycle

- `create_escrow(env, depositor, beneficiary, token, amount, schema_id, required_issuer, deadline) -> u64`
  - `depositor.require_auth()`.
  - Transfers `amount` of `token` from depositor into this contract.
  - Returns a strictly increasing escrow id.

- `claim(env, escrow_id, beneficiary, credential_id)`
  - `beneficiary.require_auth()`.
  - Cross-contract call to `vc-verifier::verify`.
  - Transfers full amount to beneficiary on success.

- `refund(env, escrow_id, depositor)`
  - `depositor.require_auth()`.
  - Transfers full amount to depositor after the deadline.

- `get_escrow(env, escrow_id) -> Escrow`
  - Unknown id panics with `CommonError::NotFound` (code 2).

### Metadata

- `version(env) -> Symbol`
  - Returns `"0_1_0"`.

## Error codes

Shared codes (`registry_core::CommonError`):

| Code | Variant | When |
| --- | --- | --- |
| 1 | AlreadyInitialized | Second `initialize` |
| 2 | NotFound | Unknown `escrow_id` |
| 4 | NotInitialized | Call before `initialize` |

Contract-specific codes:

| Code | Variant | When |
| --- | --- | --- |
| 10 | InvalidAmount | `amount <= 0` |
| 11 | DeadlineInPast | `deadline <= now` at creation |
| 12 | EscrowNotFunded | State is `Claimed` or `Refunded` |
| 13 | DeadlinePassed | `claim` at or after deadline |
| 14 | DeadlineNotReached | `refund` before deadline |
| 15 | CredentialNotValid | `verify` returned invalid |
| 16 | NotBeneficiary | Wrong caller on `claim` |
| 17 | NotDepositor | Wrong caller on `refund` |
| 18 | SelfEscrow | `beneficiary == depositor` |

## Events

- `EscrowCreated { escrow_id, depositor, beneficiary, token, amount }`
- `EscrowClaimed { escrow_id, beneficiary, depositor, token, amount }`
- `EscrowRefunded { escrow_id, depositor, beneficiary, token, amount }`

## Testing

Unit tests use a real Stellar Asset Contract and a live `vc-verifier` wired to issuer, schema, and revocation registries.

```bash
cargo test -p credential-escrow-contract
```

Property tests cover settlement invariants; fuzz target:

```bash
cd contracts/credential-escrow
cargo +nightly fuzz run fuzz_escrow_settlement --sanitizer none -- -max_total_time=60
```
