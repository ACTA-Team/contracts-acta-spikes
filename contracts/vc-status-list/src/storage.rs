//! Storage layout and helpers.
//!
//! Persistent storage is organized as:
//!   `Metadata(issuer, list_id)` → list metadata (size, chunk count)
//!   `Chunk(issuer, list_id, chunk_idx)` → raw bitmap bytes (CHUNK_SIZE each)
//!
//! Reads never extend TTL (verifiers may call `is_revoked` frequently).
//! Only writes (`set_status` / `set_status_batch`) extend TTL on the modified chunk.

use soroban_sdk::{contracttype, Address, Bytes, Env, Symbol};

/// Chunk size in bytes (4 KB = 32 768 bits).
pub const CHUNK_SIZE: u32 = 4096;

/// Maximum allowed list size in bits (1 048 576 bits = 128 KB = 32 chunks).
pub const MAX_LIST_SIZE: u32 = 1_048_576;

// TTL constants (~5 s ledger close): 518_400 ≈ 30 days, 3_110_400 ≈ 180 days.
const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 3_110_400;

/// Storage keys separated by role (explicit role isolation).
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// List metadata keyed by (issuer, list_id) → persistent.
    Metadata(Address, Symbol),
    /// Chunk of bitmap bytes keyed by (issuer, list_id, chunk_index) → persistent.
    Chunk(Address, Symbol, u32),
}

/// On-chain metadata for a status list.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListMetadata {
    /// Total number of bits in this list.
    pub size: u32,
    /// Number of chunks allocated: ceil(size / (CHUNK_SIZE * 8)).
    pub num_chunks: u32,
}

// --- Metadata helpers ---

/// Check if a list exists for the given issuer and list_id.
pub fn has_list(e: &Env, issuer: &Address, list_id: &Symbol) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::Metadata(issuer.clone(), list_id.clone()))
}

/// Read list metadata. Returns `None` if the list does not exist.
pub fn read_metadata(e: &Env, issuer: &Address, list_id: &Symbol) -> Option<ListMetadata> {
    e.storage()
        .persistent()
        .get(&DataKey::Metadata(issuer.clone(), list_id.clone()))
}

/// Write list metadata.
pub fn write_metadata(e: &Env, issuer: &Address, list_id: &Symbol, meta: &ListMetadata) {
    let key = DataKey::Metadata(issuer.clone(), list_id.clone());
    e.storage().persistent().set(&key, meta);
    e.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

// --- Chunk helpers ---

/// Read a chunk of the bitmap. Returns empty bytes if the chunk has never
/// been written (all zeros implied).
pub fn read_chunk(e: &Env, issuer: &Address, list_id: &Symbol, chunk_idx: u32) -> Bytes {
    let key = DataKey::Chunk(issuer.clone(), list_id.clone(), chunk_idx);
    e.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Bytes::new(e))
}

/// Write a chunk of the bitmap and extend its TTL.
pub fn write_chunk(
    e: &Env,
    issuer: &Address,
    list_id: &Symbol,
    chunk_idx: u32,
    data: &Bytes,
) {
    let key = DataKey::Chunk(issuer.clone(), list_id.clone(), chunk_idx);
    e.storage().persistent().set(&key, data);
    e.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

/// Compute the chunk index for a given bit index.
#[inline]
pub fn chunk_for_bit(bit_index: u32) -> u32 {
    let byte_offset = bit_index / 8;
    byte_offset / CHUNK_SIZE
}

/// Compute the byte offset within a chunk for a given bit index.
#[inline]
pub fn byte_in_chunk(bit_index: u32) -> u32 {
    let byte_offset = bit_index / 8;
    byte_offset % CHUNK_SIZE
}

/// Compute the number of chunks needed for a given size in bits.
#[inline]
pub fn num_chunks(size: u32) -> u32 {
    let total_bytes = (size + 7) / 8; // round up to whole bytes
    (total_bytes + CHUNK_SIZE - 1) / CHUNK_SIZE
}
