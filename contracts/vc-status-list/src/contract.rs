//! Contract entry points for vc-status-list.
//!
//! Implements the W3C StatusList2021 pattern: each issuer maintains one or more
//! bitmap "status lists". A credential carries an index into a list and
//! revocation flips a single bit — privacy-preserving and scalable.

use crate::error::ContractError;
use crate::events;
use crate::storage::{self, ListMetadata};
use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, Address, Bytes, Env, Symbol,
    Vec,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

contractmeta!(
    key = "Description",
    val = "VC Status List: W3C StatusList2021 bitmap revocation at scale",
);

#[contract]
pub struct VcStatusListContract;

#[contractimpl]
impl VcStatusListContract {
    // -----------------------------------------------------------------------
    // List creation
    // -----------------------------------------------------------------------

    /// Create a new status list of `size` bits (rounded up to whole bytes).
    /// Only the `issuer` address may create lists under its own identity.
    ///
    /// # Arguments
    /// * `issuer` - The address that owns the list. Must authorize the call.
    /// * `list_id` - A symbolic identifier for the list (e.g. `Symbol::new("main")`).
    /// * `size` - Number of bits in the list. Capped at [`storage::MAX_LIST_SIZE`].
    ///
    /// # Errors
    /// * `ListAlreadyExists` - if `(issuer, list_id)` already exists.
    /// * `SizeTooLarge` - if `size` exceeds `MAX_LIST_SIZE`.
    /// * `SizeZero` - if `size == 0`.
    pub fn create_list(e: Env, issuer: Address, list_id: Symbol, size: u32) {
        issuer.require_auth();

        if size == 0 {
            panic_with_error!(&e, ContractError::SizeZero);
        }
        if size > storage::MAX_LIST_SIZE {
            panic_with_error!(&e, ContractError::SizeTooLarge);
        }
        if storage::has_list(&e, &issuer, &list_id) {
            panic_with_error!(&e, ContractError::ListAlreadyExists);
        }

        let num_chunks = storage::num_chunks(size);
        let meta = ListMetadata { size, num_chunks };

        // Pre-allocate all chunks as empty so reads for any chunk succeed.
        let empty = Bytes::new(&e);
        for i in 0..num_chunks {
            let key = storage::DataKey::Chunk(issuer.clone(), list_id.clone(), i);
            e.storage().persistent().set(&key, &empty);
        }

        storage::write_metadata(&e, &issuer, &list_id, &meta);
        events::list_created(&e, &issuer, &list_id, size);
    }

    // -----------------------------------------------------------------------
    // Status updates (issuer-only write paths)
    // -----------------------------------------------------------------------

    /// Flip a single bit. Only the owning issuer may call this.
    ///
    /// # Arguments
    /// * `issuer` - The address that owns the list. Must authorize the call.
    /// * `list_id` - The symbolic identifier for the list.
    /// * `index` - The bit position (0-indexed) to flip.
    /// * `revoked` - `true` to set the bit (revoke), `false` to clear it.
    ///
    /// # Errors
    /// * `ListNotFound` - if no list exists for `(issuer, list_id)`.
    /// * `IndexOutOfRange` - if `index >= list.size`.
    pub fn set_status(
        e: Env,
        issuer: Address,
        list_id: Symbol,
        index: u32,
        revoked: bool,
    ) {
        issuer.require_auth();
        let meta = require_list(&e, &issuer, &list_id);

        if index >= meta.size {
            panic_with_error!(&e, ContractError::IndexOutOfRange);
        }

        flip_bit(&e, &issuer, &list_id, index, revoked);
        events::status_changed(&e, &issuer, &list_id, index, revoked);
    }

    /// Flip many indices in a single transaction — the key scalability
    /// advantage over the per-credential-key revocation registry.
    ///
    /// # Arguments
    /// * `issuer` - The address that owns the list. Must authorize the call.
    /// * `list_id` - The symbolic identifier for the list.
    /// * `indices` - A vector of bit positions to flip.
    /// * `revoked` - `true` to set (revoke), `false` to clear.
    ///
    /// # Errors
    /// * `ListNotFound` - if no list exists for `(issuer, list_id)`.
    /// * `IndexOutOfRange` - if any index exceeds `list.size - 1`.
    pub fn set_status_batch(
        e: Env,
        issuer: Address,
        list_id: Symbol,
        indices: Vec<u32>,
        revoked: bool,
    ) {
        issuer.require_auth();
        let meta = require_list(&e, &issuer, &list_id);

        // Validate all indices first (fail-fast: no partial writes).
        for i in 0..indices.len() {
            let idx = indices.get_unchecked(i);
            if idx >= meta.size {
                panic_with_error!(&e, ContractError::IndexOutOfRange);
            }
        }

        let count = indices.len();
        for i in 0..count {
            let idx = indices.get_unchecked(i);
            flip_bit(&e, &issuer, &list_id, idx, revoked);
        }

        events::status_batch_changed(&e, &issuer, &list_id, count, revoked);
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    /// Hot path for verifiers: is bit `index` set?
    ///
    /// Returns `false` (not an error) when the list does not exist or the index
    /// is out of range — a verifier should fail-open only if the list is known.
    ///
    /// # Arguments
    /// * `issuer` - The address that owns the list.
    /// * `list_id` - The symbolic identifier for the list.
    /// * `index` - The bit position to check.
    ///
    /// # Returns
    /// `true` if the bit is set (revoked), `false` otherwise.
    pub fn is_revoked(e: Env, issuer: Address, list_id: Symbol, index: u32) -> bool {
        let meta = match storage::read_metadata(&e, &issuer, &list_id) {
            Some(m) => m,
            None => return false,
        };
        if index >= meta.size {
            return false;
        }
        read_bit(&e, &issuer, &list_id, index)
    }

    /// Return a chunk of the raw bitmap so verifiers can cache it off-chain.
    ///
    /// If the chunk has never been written, returns an empty `Bytes` (all
    /// zeros implied). If the list does not exist, panics with `ListNotFound`.
    ///
    /// # Arguments
    /// * `issuer` - The address that owns the list.
    /// * `list_id` - The symbolic identifier for the list.
    /// * `chunk` - The 0-indexed chunk number to retrieve.
    ///
    /// # Errors
    /// * `ListNotFound` - if no list exists for `(issuer, list_id)`.
    pub fn get_chunk(e: Env, issuer: Address, list_id: Symbol, chunk: u32) -> Bytes {
        let meta = require_list(&e, &issuer, &list_id);
        if chunk >= meta.num_chunks {
            return Bytes::new(&e);
        }
        storage::read_chunk(&e, &issuer, &list_id, chunk)
    }

    /// Returns the list metadata (size, num_chunks) for the given list.
    ///
    /// # Errors
    /// * `ListNotFound` - if no list exists for `(issuer, list_id)`.
    pub fn get_list_metadata(e: Env, issuer: Address, list_id: Symbol) -> ListMetadata {
        require_list(&e, &issuer, &list_id)
    }

    /// Returns true if a list exists for the given issuer and list_id.
    pub fn list_exists(e: Env, issuer: Address, list_id: Symbol) -> bool {
        storage::has_list(&e, &issuer, &list_id)
    }

    /// Returns the contract version string.
    pub fn version(e: Env) -> soroban_sdk::String {
        soroban_sdk::String::from_str(&e, VERSION)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Checks that a list exists and returns its metadata. Panics with
/// `ListNotFound` if it does not.
fn require_list(e: &Env, issuer: &Address, list_id: &Symbol) -> ListMetadata {
    storage::read_metadata(e, issuer, list_id)
        .unwrap_or_else(|| panic_with_error!(e, ContractError::ListNotFound))
}

/// Read a single bit from the bitmap.
fn read_bit(e: &Env, issuer: &Address, list_id: &Symbol, index: u32) -> bool {
    let chunk_idx = storage::chunk_for_bit(index);
    let byte_off = storage::byte_in_chunk(index);
    let bit_off = index % 8;

    let chunk = storage::read_chunk(e, issuer, list_id, chunk_idx);

    let byte = match chunk.get(byte_off) {
        Some(b) => b,
        None => return false,
    };

    (byte & (1 << (7 - bit_off))) != 0
}

/// Flip a single bit in the bitmap (set or clear). Reads the chunk, modifies
/// the byte, and writes it back. Extends TTL on the modified chunk.
fn flip_bit(e: &Env, issuer: &Address, list_id: &Symbol, index: u32, set: bool) {
    let chunk_idx = storage::chunk_for_bit(index);
    let byte_off = storage::byte_in_chunk(index);
    let bit_off = index % 8;
    let mask: u8 = 1 << (7 - bit_off);

    let chunk = storage::read_chunk(e, issuer, list_id, chunk_idx);
    let chunk_len = chunk.len();

    // Get current byte value or 0 if beyond current length
    let current_byte = if byte_off < chunk_len {
        chunk.get_unchecked(byte_off)
    } else {
        0u8
    };

    let new_byte = if set {
        current_byte | mask
    } else {
        current_byte & !mask
    };

    // Reuse the existing chunk and modify the target byte in-place
    let mut updated = chunk;
    while updated.len() <= byte_off {
        updated.push_back(0u8);
    }
    updated.set(byte_off, new_byte);

    storage::write_chunk(e, issuer, list_id, chunk_idx, &updated);
}
