# vc-issuer-registry

`vc-issuer-registry` is a Soroban smart contract that stores an allowlist of Verifiable Credential issuers and a small amount of issuer metadata. It is intentionally focused on one question: “is this address allowed to issue credentials?”

## Purpose and threat model

This contract is designed for a simple V1 governance model:

- The admin address is the single authority for issuer registration and updates.
- The admin can add issuers, remove issuers, enable or disable issuers, and edit metadata.
- The contract does not implement multisig, timelocks, DAO voting, or identity verification.
- If the admin key is compromised, the allowlist can be changed arbitrarily.

In practice, this contract should be treated as a policy primitive. A higher-level flow such as `vc-vault` should check this registry before allowing issuance to continue.

## Storage layout

| Key | Storage type | Description |
| --- | --- | --- |
| `Admin` | Instance | Contract admin address |
| `Issuer(Address)` | Persistent | `IssuerRecord` for each registered issuer |

### IssuerRecord

```rust
pub struct IssuerRecord {
    pub allowed: bool,
    pub name: Option<Symbol>,
    pub did: Option<Bytes>,
    pub url: Option<Bytes>,
}
```

## Public API reference

### Initialization

- `initialize(env, admin: Address)`
  - One-time initialization.
  - Stores the admin and emits `Initialized`.
  - Repeated calls panic with `AlreadyInitialized`.

### Issuer management

- `add_issuer(env, issuer: Address, name: Option<Symbol>, did: Option<Bytes>, url: Option<Bytes>)`
  - Requires admin authentication.
  - Adds a new issuer with `allowed = true`.
  - Fails if the issuer already exists (`IssuerAlreadyExists`).

- `set_issuer_metadata(env, issuer: Address, name: Option<Symbol>, did: Option<Bytes>, url: Option<Bytes>)`
  - Requires admin authentication.
  - Updates only `name`, `did`, and `url`.
  - Preserves the current `allowed` value and fails if the issuer is missing.

- `set_issuer_allowed(env, issuer: Address, allowed: bool)`
  - Requires admin authentication.
  - Enables or disables an issuer without deleting the record.

- `remove_issuer(env, issuer: Address)`
  - Requires admin authentication.
  - Deletes the issuer record entirely.
  - Fails if the issuer is not registered.

### Queries

- `get_issuer(env, issuer: Address) -> IssuerRecord`
  - Returns the full record or panics with `IssuerNotFound`.

- `is_issuer_allowed(env, issuer: Address) -> bool`
  - Returns `true` only when the issuer exists and its `allowed` flag is `true`.
  - Unknown or removed issuers return `false`.

- `admin(env) -> Address`
  - Returns the current admin address or panics with `NotInitialized`.

- `version(env) -> String`
  - Returns the crate version string.

## Storage semantics for removal

`remove_issuer` performs a hard delete of the issuer record. After removal:

- `is_issuer_allowed` returns `false`.
- `get_issuer` panics with `IssuerNotFound`.
- The same address can be re-added later with `add_issuer`.

If you want a soft disable instead of a hard delete, use `set_issuer_allowed(false)`. That keeps the record in place while preventing the issuer from being treated as allowed.

## Metadata validation

- `did` and `url` are limited to 256 bytes each.
- `add_issuer` and `set_issuer_metadata` panic with `InvalidMetadata` if either field exceeds the limit.
- `name` uses `Symbol`, so the Soroban host enforces its own bounds.

## Suggested integration pattern for `vc-vault`

Before allowing a credential issuance flow to proceed, the issuing component should:

1. Resolve the issuer address.
2. Call `is_issuer_allowed`.
3. Abort the flow if the answer is `false`.
4. Optionally call `get_issuer` to read metadata for audit or UI purposes.

That keeps issuer policy centralized and makes future upgrades easier.

## Error codes

| Code | Variant | Meaning |
| --- | --- | --- |
| 1 | `AlreadyInitialized` | `initialize` called more than once |
| 2 | `IssuerNotFound` | Issuer not in registry |
| 3 | `IssuerAlreadyExists` | Issuer already registered |
| 4 | `NotInitialized` | Contract not yet initialized |
| 5 | `InvalidMetadata` | Metadata field exceeds max byte length |

## Events

| Event | Emitted by | Fields |
| --- | --- | --- |
| `Initialized` | `initialize` | `admin: Address` |
| `IssuerAdded` | `add_issuer` | `issuer: Address` |
| `MetadataUpdated` | `set_issuer_metadata` | `issuer: Address` |
| `IssuerAllowedSet` | `set_issuer_allowed` | `issuer: Address, allowed: bool` |
| `IssuerRemoved` | `remove_issuer` | `issuer: Address` |

## CLI examples

These snippets assume that the contract is already deployed, the admin identity is funded, and the relevant environment variables are set.

```bash
export CONTRACT_ID="<deployed-contract-id>"
export ADMIN_ADDRESS="<admin-address>"
export ADMIN_SECRET="<admin-secret>"
export ISSUER_ADDRESS="<issuer-address>"
export NETWORK="testnet"
```

### 1. Initialize the contract

```bash
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network "$NETWORK" \
  -- initialize \
  --admin "$ADMIN_ADDRESS"
```

### 2. Add an issuer

```bash
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network "$NETWORK" \
  -- add_issuer \
  --issuer "$ISSUER_ADDRESS" \
  --name "ExampleIssuer" \
  --did "did:example:issuer-1" \
  --url "https://issuer.example"
```

### 3. Query whether an issuer is allowed

```bash
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --network "$NETWORK" \
  -- is_issuer_allowed \
  --issuer "$ISSUER_ADDRESS"
```

### 4. Disable an issuer without deleting it

```bash
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network "$NETWORK" \
  -- set_issuer_allowed \
  --issuer "$ISSUER_ADDRESS" \
  --allowed false
```

## Build and test

```bash
# from the repository root
cargo test --workspace

# from contracts/vc-issuer-registry
soroban contract build
```
