# vc-health-license-registry

`vc-health-license-registry` is a Soroban contract that tracks professional licenses whose validity changes with time. A license expires on a date, can be temporarily suspended by the issuing authority, and can be permanently revoked. Status is **derived at read time** from the stored record plus `env.ledger().timestamp()` — it is never persisted as a field.

## Purpose

Every registry in this repo today stores a boolean or a record and answers "does it exist?". Professional licensing (medical boards, nursing councils, pharmacy registries) needs a credential whose validity **changes with time on its own**. A verifier asking "is this doctor licensed right now?" must get the correct answer without anyone submitting a transaction when the clock passes a deadline.

Issuance is gated by a live cross-contract check against `vc-issuer-registry`.

## Storage layout

| Key | Storage type | Description |
| --- | --- | --- |
| `Admin` (registry-core) | Instance | Contract admin |
| `IssuerRegistry` | Instance | `vc-issuer-registry` contract address |
| `License(BytesN<32>)` | Persistent | License record keyed by deterministic id |

### License

```rust
pub struct License {
    pub authority: Address,
    pub holder: Address,
    pub specialty: Symbol,
    pub jurisdiction: Symbol,
    pub issued_at: u64,
    pub expires_at: u64,
    pub suspended_until: u64, // 0 = not suspended
    pub revoked: bool,
    pub metadata: Bytes,
}
```

### Status precedence (derived, never stored)

1. `revoked == true` → **Revoked**
2. `now >= expires_at` → **Expired**
3. `suspended_until > now` → **Suspended**
4. otherwise → **Active**

Revocation is terminal. Expiry outranks suspension (a suspended license past its deadline is `Expired`, not `Suspended`). Suspension self-clears when `now >= suspended_until` without any transaction.

## Public API

### Initialization

- `initialize(env, admin: Address, issuer_registry: Address)`
  - One-time setup. Stores admin and issuer registry address.
  - Second call panics with `CommonError::AlreadyInitialized` (code 1).

### License identity

- `license_id(env, authority, holder, specialty, jurisdiction) -> BytesN<32>`
  - `sha256(xdr(authority) || xdr(holder) || xdr(specialty) || xdr(jurisdiction))`

### Issuance

- `issue_license(env, authority, holder, specialty, jurisdiction, expires_at, metadata) -> BytesN<32>`
  - `authority.require_auth()`. Live `is_issuer_allowed(authority)` via cross-contract call.
  - `expires_at` must be strictly greater than current ledger timestamp.
  - `metadata` bounded by `registry_core::DEFAULT_MAX_BYTES` (256).
  - Fails on duplicate `(authority, holder, specialty, jurisdiction)`.

### Mutations (authority-scoped)

- `renew_license(env, authority, license_id, new_expires_at)`
  - `new_expires_at > max(now, current expires_at)`.

- `suspend_license(env, authority, license_id, until, reason)`
  - `until > now`. `reason` is emitted in the event only (not stored on the record).

- `lift_suspension(env, authority, license_id)`
  - Fails unless the license is currently suspended (`suspended_until > now`).

- `revoke_license(env, authority, license_id)`
  - Terminal. Sets `revoked = true`.

### Queries

- `get_license(env, license_id) -> License`
- `license_status(env, license_id) -> LicenseStatus`
  - Pure function of stored record and `env.ledger().timestamp()`.
- `is_license_valid(env, license_id) -> bool`
  - True when status is `Active`.
- `version(env) -> Symbol`
  - Returns `"0_1_0"`.

## Error codes

Shared codes (`registry_core::CommonError`):

| Code | Variant | When |
| --- | --- | --- |
| 1 | AlreadyInitialized | Second `initialize` |
| 2 | NotFound | Unknown `license_id` |
| 3 | AlreadyExists | Duplicate issue |
| 4 | NotInitialized | Call before `initialize` |
| 5 | Unauthorized | Caller is not the stored authority |
| 6 | InvalidInput | Oversized `metadata` |

Contract-specific codes:

| Code | Variant | When |
| --- | --- | --- |
| 10 | AuthorityNotAllowed | `is_issuer_allowed(authority)` returned false |
| 11 | ExpiryInPast | `expires_at <= now` at issue time |
| 12 | RenewalNotMonotonic | `new_expires_at <= max(now, expires_at)` |
| 13 | LicenseRevoked | Mutation on a revoked license |
| 14 | NotSuspended | `lift_suspension` when not suspended |
| 15 | SuspensionInPast | `until <= now` at suspend time |

## Events

| Event | Emitted by | Fields |
| --- | --- | --- |
| `LicenseIssued` | `issue_license` | `license_id`, `authority` |
| `LicenseRenewed` | `renew_license` | `license_id`, `authority` |
| `LicenseSuspended` | `suspend_license` | `license_id`, `authority`, `until`, `reason` |
| `SuspensionLifted` | `lift_suspension` | `license_id`, `authority` |
| `LicenseRevoked` | `revoke_license` | `license_id`, `authority` |

## Testing

```bash
cargo test -p vc-health-license-registry-contract
cd contracts/vc-health-license-registry && cargo +nightly fuzz run fuzz_license_lifecycle --sanitizer none -- -max_total_time=60
```

Property tests cover revocation monotonicity, non-decreasing expiry, status purity at a fixed timestamp, and license id determinism/collision resistance.
