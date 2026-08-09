//! Contract entry points for did-stellar-registry.

use crate::error::ContractError;
use crate::events;
use crate::storage::{self, DidRecord};
use soroban_sdk::{
    contract, contractimpl, contractmeta, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum allowed byte length for an inline DID document.
/// A W3C DID document with id + verificationMethod + authentication fits
/// comfortably in ~400 bytes; 1024 gives headroom for a service endpoint.
const MAX_DOCUMENT_BYTES: u32 = 1024;

contractmeta!(
    key = "Description",
    val = "DID Stellar Registry: self-sovereign DID registry for the did:stellar method",
);

#[contract]
pub struct DidStellarRegistryContract;

#[contractimpl]
impl DidStellarRegistryContract {
    // -----------------------------------------------------------------------
    // Write operations (controller-only, no admin)
    // -----------------------------------------------------------------------

    /// Register a new DID for `controller`.
    ///
    /// The DID identifier is derived deterministically as
    /// `sha256(xdr(controller))` and returned to the caller as `Bytes`.
    /// This identifier is stable: the same address always produces the same
    /// DID, on-chain and off-chain alike.
    ///
    /// Panics with `DocumentTooLarge` if `document.len() > 1024`.
    /// Panics with `DidAlreadyExists` if a DID for this controller is already
    /// registered (including deactivated ones).
    pub fn register(e: Env, controller: Address, document: Bytes) -> Bytes {
        validate_document(&e, &document);
        controller.require_auth();

        let did_id = compute_did(&e, &controller);

        if storage::has_did(&e, &did_id) {
            panic_with_error!(&e, ContractError::DidAlreadyExists);
        }

        let now = e.ledger().timestamp();
        let record = DidRecord {
            controller: controller.clone(),
            document,
            version: 1,
            deactivated: false,
            created_at: now,
            updated_at: now,
        };
        storage::write_did(&e, &did_id, &record);
        storage::extend_instance_ttl(&e);
        events::did_registered(&e, &did_id, &controller);

        did_id.into()
    }

    /// Update the DID document for `controller`.
    ///
    /// Increments `version` and sets `updated_at` to the current ledger
    /// timestamp. Only the controller that originally registered the DID
    /// may call this.
    ///
    /// Panics with `DocumentTooLarge` if `document.len() > 1024`.
    /// Panics with `DidNotFound` if no DID exists for this controller.
    /// Panics with `DidDeactivated` if the DID has been deactivated.
    pub fn update(e: Env, controller: Address, document: Bytes) {
        validate_document(&e, &document);
        controller.require_auth();

        let did_id = compute_did(&e, &controller);

        let mut record = storage::read_did(&e, &did_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::DidNotFound));

        if record.deactivated {
            panic_with_error!(&e, ContractError::DidDeactivated);
        }

        record.document = document;
        record.version += 1;
        record.updated_at = e.ledger().timestamp();

        storage::write_did(&e, &did_id, &record);
        storage::extend_instance_ttl(&e);
        events::did_updated(&e, &did_id, &controller, record.version);
    }

    /// Deactivate the DID belonging to `controller`.
    ///
    /// The record is **never deleted** — it remains readable via `resolve` with
    /// `deactivated = true`. A deactivated DID cannot be updated or re-registered.
    ///
    /// Panics with `DidNotFound` if no DID exists for this controller.
    /// Panics with `DidDeactivated` if the DID is already deactivated.
    pub fn deactivate(e: Env, controller: Address) {
        controller.require_auth();

        let did_id = compute_did(&e, &controller);

        let mut record = storage::read_did(&e, &did_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::DidNotFound));

        if record.deactivated {
            panic_with_error!(&e, ContractError::DidDeactivated);
        }

        record.deactivated = true;
        record.updated_at = e.ledger().timestamp();

        storage::write_did(&e, &did_id, &record);
        events::did_deactivated(&e, &did_id, &controller);
    }

    // -----------------------------------------------------------------------
    // Read-only queries (permissionless)
    // -----------------------------------------------------------------------

    /// Returns the full `DidRecord` for the given DID bytes.
    ///
    /// Panics with `DidNotFound` if the DID does not exist or if `did` is
    /// not a valid 32-byte identifier.
    pub fn resolve(e: Env, did: Bytes) -> DidRecord {
        storage::extend_instance_ttl(&e);
        let did_id = bytes_to_did_id(&e, &did);
        storage::read_did(&e, &did_id)
            .unwrap_or_else(|| panic_with_error!(&e, ContractError::DidNotFound))
    }

    /// Returns `true` if the given DID exists and is not deactivated.
    /// Returns `false` for unknown DIDs or invalid byte lengths (no panic).
    pub fn is_active(e: Env, did: Bytes) -> bool {
        storage::extend_instance_ttl(&e);
        if did.len() != 32 {
            return false;
        }
        let Ok(did_id) = BytesN::<32>::try_from(did) else {
            return false;
        };
        storage::read_did(&e, &did_id)
            .map(|r| !r.deactivated)
            .unwrap_or(false)
    }

    /// Computes and returns the DID identifier for a given controller address
    /// without writing to storage. Useful for off-chain pre-computation or
    /// UI tooling that needs to predict the DID before submitting a transaction.
    pub fn did_for(e: Env, controller: Address) -> Bytes {
        compute_did(&e, &controller).into()
    }

    /// Returns the contract version string (taken from `Cargo.toml` at compile time).
    pub fn version(e: Env) -> soroban_sdk::String {
        soroban_sdk::String::from_str(&e, VERSION)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Computes `sha256(xdr(controller))`.
///
/// XDR encoding is used so the preimage is unambiguous and matches what
/// off-chain tooling produces when serializing the same `ScVal::Address`.
fn compute_did(e: &Env, controller: &Address) -> BytesN<32> {
    let mut preimage = Bytes::new(e);
    preimage.append(&controller.clone().to_xdr(e));
    e.crypto().sha256(&preimage).to_bytes()
}

/// Converts a `Bytes` value to `BytesN<32>`, panicking with `DidNotFound`
/// if the length is not exactly 32.
fn bytes_to_did_id(e: &Env, did: &Bytes) -> BytesN<32> {
    BytesN::<32>::try_from(did.clone())
        .unwrap_or_else(|_| panic_with_error!(e, ContractError::DidNotFound))
}

/// Panics with `DocumentTooLarge` if the document exceeds MAX_DOCUMENT_BYTES.
fn validate_document(e: &Env, document: &Bytes) {
    if document.len() > MAX_DOCUMENT_BYTES {
        panic_with_error!(e, ContractError::DocumentTooLarge);
    }
}
