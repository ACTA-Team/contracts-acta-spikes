# VC Status List Contract

Soroban contract implementing the [W3C StatusList2021](https://www.w3.org/TR/vc-status-list/)
pattern for scalable, privacy-preserving credential revocation.

## Overview

Each issuer maintains one or more bitmap "status lists". A verifiable credential
carries an `(issuer, list_id, index)` triple; revocation flips a single bit in the
issuer's list. Because an index reveals nothing on its own, the approach is
**privacy-preserving** — unlike per-credential on-chain keys that publicly
enumerate every revoked credential identifier.

### Why a bitmap instead of per-credential entries?

The existing `vc-revocation-registry` stores one persistent ledger entry per
revoked credential. That works for tens of revocations but does not scale to
tens of thousands. With a bitmapped status list:

- **Batch revocation** is a single `set_status_batch` call, not N separate writes.
- **Rent** is per 4 KB chunk, not per credential.
- **TTL** is extended per chunk on writes — no per-credential TTL to manage.

## API

| Function | Access | Description |
|---|---|---|
| `create_list(issuer, list_id, size)` | Issuer only | Create a new bitmap of `size` bits |
| `set_status(issuer, list_id, index, revoked)` | Issuer only | Flip a single bit |
| `set_status_batch(issuer, list_id, indices, revoked)` | Issuer only | Flip many bits in one TX |
| `is_revoked(issuer, list_id, index) -> bool` | Public | Check if a bit is set |
| `get_chunk(issuer, list_id, chunk) -> Bytes` | Public | Retrieve raw bitmap bytes |
| `get_list_metadata(issuer, list_id) -> ListMetadata` | Public | Get list size & chunk count |
| `list_exists(issuer, list_id) -> bool` | Public | Check if a list exists |
| `version() -> String` | Public | Contract version |

## Bit Layout

Each list is a bitstring indexed from 0 (MSB of byte 0) to `size - 1` (LSB of
last byte). Bits are stored MSB-first within each byte: index 0 is bit 7
(0x80) of byte 0, index 1 is bit 6 (0x40), and so on.

```
Byte 0:  [b7 b6 b5 b4 b3 b2 b1 b0]
          ↑                    ↑
        index 0             index 7

Byte 1:  [b7 b6 b5 b4 b3 b2 b1 b0]
          ↑                    ↑
        index 8             index 15
```

A set bit (1) means **revoked**; a clear bit (0) means **not revoked**.

## Chunked Storage

The bitmap is split into 4 KB chunks stored separately on-chain:

- **Chunk size:** 4,096 bytes = 32,768 bits
- **Max list size:** 1,048,576 bits = 128 KB = 32 chunks
- **Storage key:** `DataKey::Chunk(issuer, list_id, chunk_index)`

A single-bit update only reads and writes **one** 4 KB chunk, not the entire list.

Per-chunk TTL is extended to ~180 days on every write. Reads do not extend TTL.

## Off-Chain Verifier Integration

A verifier should:

1. Call `get_chunk(issuer, list_id, 0)` (and subsequent chunks as needed)
   to obtain the full bitmap.
2. Cache the result off-chain (e.g., in memory or a local database).
3. Check `is_revoked()` by computing the byte and bit offset:

```
byte_offset = index / 8
bit_offset  = index % 8
mask        = 1 << (7 - bit_offset)
revoked     = (chunk_bytes[byte_offset] & mask) != 0
```

4. Re-fetch and refresh the cache periodically based on the desired freshness
   guarantee (e.g., every few minutes or on every verification, depending on
   security requirements).

## Authorization

Authorization is **per-issuer**, not admin-only. An issuer owns its own lists
and must authorize (`require_auth`) every write call. This differs from
`vc-revocation-registry`, where a contract admin manages all revocations.

## Relationship to vc-revocation-registry

This contract is not a replacement for `vc-revocation-registry` — it is the
**bulk/scale path**:

- **Small issuers** with few credentials can continue using the direct
  per-credential registry.
- **Large issuers** with tens of thousands of credentials should use
  `vc-status-list` for efficient batch revocation.

A verifier should consult whichever revocation mechanism the issuer has
registered.

## Constraints

| Parameter | Value |
|---|---|
| Minimum list size | 1 bit |
| Maximum list size | 1,048,576 bits |
| Chunk size | 4,096 bytes (32,768 bits) |
| Maximum chunks | 32 |

## Errors

| Code | Name | Description |
|---|---|---|
| 1 | `ListAlreadyExists` | The list has already been created for the issuer/list_id |
| 2 | `ListNotFound` | No list exists for the given issuer and list_id |
| 3 | `IndexOutOfRange` | The bit index exceeds the list size |
| 4 | `SizeTooLarge` | The requested list size exceeds the maximum |
| 5 | `SizeZero` | The requested size is zero |

## Build & Test

```bash
# Build
cargo build -p vc-status-list-contract

# Test
cargo test -p vc-status-list-contract
```
