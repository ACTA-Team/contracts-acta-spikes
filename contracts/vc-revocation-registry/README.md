# VC Revocation Registry Contract

An on-chain registry to track revoked Verifiable Credentials, allowing authorized issuers to revoke their own VCs and enabling verifiers to check revocation status without an off-chain indexer.

## Overview

This contract complements the `vc-issuer-registry` and is a foundational piece of the VC lifecycle. It provides:

- **Revocation tracking**: Mark VCs as revoked with issuer identity and timestamp
- **Batch operations**: Revoke multiple VCs in a single transaction
- **Query interface**: Check revocation status and retrieve revocation details
- **Admin controls**: Admin can unrevoke VCs as a safeguard

## Storage Layout

| Key | Storage Type | Description |
|-----|--------------|-------------|
| `Admin` | Instance | Contract admin address |
| `Revoked(BytesN<32>)` | Persistent | `RevocationRecord` per VC id |

### RevocationRecord

```rust
pub struct RevocationRecord {
    pub issuer: Address,      // Issuer who revoked the VC
    pub revoked_at: u64,      // Ledger timestamp
    pub reason: Option<Symbol>, // Optional revocation reason
}
```

## Entry Points

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(admin)` | admin | One-time init; stores admin |
| `revoke(issuer, vc_id, reason)` | issuer | Mark a VC as revoked |
| `batch_revoke(issuer, vc_ids, reason)` | issuer | Revoke multiple VCs in one tx |
| `unrevoke(vc_id)` | admin | Remove a revocation entry (admin-only) |
| `is_revoked(vc_id)` | — | Returns `true` if VC is revoked |
| `get_revocation(vc_id)` | — | Returns `RevocationRecord` or panics |
| `admin()` | — | Returns current admin address |
| `version()` | — | Returns crate version string |

## Error Codes

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `AlreadyInitialized` | `initialize` called more than once |
| 2 | `NotInitialized` | Contract not yet initialized |
| 3 | `AlreadyRevoked` | VC already in revocation list |
| 4 | `NotRevoked` | VC not present in revocation list |
| 5 | `UnauthorizedIssuer` | Caller is not the authorized issuer |

## Events

| Event | Emitted by | Fields |
|-------|------------|--------|
| `Initialized` | `initialize` | `admin: Address` |
| `Revoked` | `revoke` / `batch_revoke` | `vc_id: BytesN<32>, issuer: Address` |
| `Unrevoked` | `unrevoke` | `vc_id: BytesN<32>` |

## Building

```bash
# Build the contract
cargo build -p vc-revocation-registry-contract

# Build WASM for deployment
stellar contract build
```

## Usage Example

```rust
// Initialize the contract
let admin = Address::generate(&env);
client.initialize(&admin);

// Revoke a single VC
let issuer = Address::generate(&env);
let vc_id = BytesN::<32>::from_array(&env, &[1u8; 32]);
client.revoke(&issuer, &vc_id, &None);

// Check revocation status
assert!(client.is_revoked(&vc_id));

// Get revocation details
let record = client.get_revocation(&vc_id);
assert_eq!(record.issuer, issuer);

// Batch revoke multiple VCs
let mut vc_ids = Vec::new(&env);
vc_ids.push_back(vc_id_1);
vc_ids.push_back(vc_id_2);
vc_ids.push_back(vc_id_3);
client.batch_revoke(&issuer, &vc_ids, &Some(Symbol::new(&env, "expired")));

// Admin can unrevoke if needed
client.unrevoke(&vc_id);
assert!(!client.is_revoked(&vc_id));
```

## License

MIT
