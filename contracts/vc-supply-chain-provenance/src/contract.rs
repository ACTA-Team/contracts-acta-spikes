//! Contract entry points for vc-supply-chain-provenance.

use crate::error::ContractError;
use crate::events;
use crate::storage;
use registry_core::{CommonError, DEFAULT_MAX_BYTES};
use soroban_sdk::{
    contract, contractclient, contractimpl, contractmeta, contracttype, panic_with_error, Address,
    Bytes, BytesN, Env, Symbol, Vec,
};

const VERSION_SYMBOL: &str = "0_1_0";
const MAX_HOPS: u32 = 100;
const MAX_CHAIN_PAGE: u32 = 50;

// ---------------------------------------------------------------------------
// Cross-contract binding for vc-revocation-registry (mirrors only `is_revoked`).
// ---------------------------------------------------------------------------

#[contractclient(name = "RevocationRegistryClient")]
pub trait RevocationRegistryTrait {
    fn is_revoked(env: Env, issuer: Address, credential_id: Bytes) -> bool;
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchState {
    InTransit,
    Sealed,
}

#[contracttype]
#[derive(Clone)]
pub struct CustodyHop {
    pub from: Address,
    pub to: Address,
    pub at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Batch {
    pub certifier: Address,
    pub custodian: Address,
    pub product: Symbol,
    pub origin: Symbol,
    pub state: BatchState,
    pub hops: u32,
    pub created_at: u64,
    pub sealed_at: u64,
    pub metadata: Bytes,
}

contractmeta!(
    key = "Description",
    val = "VC Supply Chain Provenance: append-only custody chain with terminal sealing",
);

#[contract]
pub struct VcSupplyChainProvenanceContract;

#[contractimpl]
impl VcSupplyChainProvenanceContract {
    /// Idempotent. Second call fails with `CommonError::AlreadyInitialized`.
    pub fn initialize(e: Env, admin: Address, revocation_registry: Address) {
        if storage::has_admin(&e) {
            panic_with_error!(&e, CommonError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&e, &admin);
        storage::write_revocation_registry(&e, &revocation_registry);
        storage::extend_instance_ttl(&e);
    }

    /// Registers a new batch. The certifier becomes the initial custodian.
    pub fn register_batch(
        e: Env,
        certifier: Address,
        batch_id: BytesN<32>,
        product: Symbol,
        origin: Symbol,
        metadata: Bytes,
    ) {
        require_initialized(&e);
        certifier.require_auth();
        registry_core::validate_max_bytes(&e, &metadata, DEFAULT_MAX_BYTES);

        if storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::AlreadyExists);
        }

        let now = e.ledger().timestamp();
        let batch = Batch {
            certifier: certifier.clone(),
            custodian: certifier.clone(),
            product: product.clone(),
            origin: origin.clone(),
            state: BatchState::InTransit,
            hops: 0,
            created_at: now,
            sealed_at: 0,
            metadata,
        };

        storage::write_batch(&e, &batch_id, &batch);
        storage::extend_instance_ttl(&e);
        events::batch_registered(&e, &batch_id, &certifier, &product, &origin);
    }

    /// Attaches or replaces a certificate while the batch is in transit.
    pub fn attach_certificate(
        e: Env,
        certifier: Address,
        batch_id: BytesN<32>,
        credential_id: Bytes,
    ) {
        require_initialized(&e);
        certifier.require_auth();
        registry_core::validate_max_bytes(&e, &credential_id, DEFAULT_MAX_BYTES);

        if !storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let batch = storage::read_batch(&e, &batch_id);
        if batch.certifier != certifier {
            panic_with_error!(&e, CommonError::Unauthorized);
        }
        if batch.state == BatchState::Sealed {
            panic_with_error!(&e, ContractError::BatchSealed);
        }

        if is_credential_revoked(&e, &batch.certifier, &credential_id) {
            panic_with_error!(&e, ContractError::CertificateRevoked);
        }

        storage::write_certificate(&e, &batch_id, &credential_id);
        storage::extend_instance_ttl(&e);
        events::certificate_attached(&e, &batch_id, &certifier, &credential_id);
    }

    /// Transfers custody to a new custodian, appending a hop.
    pub fn transfer_custody(e: Env, batch_id: BytesN<32>, from: Address, to: Address) {
        require_initialized(&e);
        from.require_auth();

        if !storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let mut batch = storage::read_batch(&e, &batch_id);

        if batch.state == BatchState::Sealed {
            panic_with_error!(&e, ContractError::BatchSealed);
        }
        if batch.custodian != from {
            panic_with_error!(&e, ContractError::NotCustodian);
        }
        if from == to {
            panic_with_error!(&e, ContractError::SelfTransfer);
        }
        if batch.hops >= MAX_HOPS {
            panic_with_error!(&e, ContractError::HopLimitExceeded);
        }

        let hop_index = batch.hops;
        let hop = CustodyHop {
            from: from.clone(),
            to: to.clone(),
            at: e.ledger().timestamp(),
        };

        storage::write_hop(&e, &batch_id, hop_index, &hop);
        batch.custodian = to.clone();
        batch.hops += 1;
        storage::write_batch(&e, &batch_id, &batch);
        storage::extend_instance_ttl(&e);
        events::custody_transferred(&e, &batch_id, &from, &to, hop_index);
    }

    /// Terminal: seals the batch. No further transfers or certificate changes.
    pub fn seal_batch(e: Env, batch_id: BytesN<32>, custodian: Address) {
        require_initialized(&e);
        custodian.require_auth();

        if !storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let mut batch = storage::read_batch(&e, &batch_id);

        if batch.custodian != custodian {
            panic_with_error!(&e, ContractError::NotCustodian);
        }
        if batch.state == BatchState::Sealed {
            panic_with_error!(&e, ContractError::BatchSealed);
        }
        if !storage::has_certificate(&e, &batch_id) {
            panic_with_error!(&e, ContractError::NoCertificateAttached);
        }

        batch.state = BatchState::Sealed;
        batch.sealed_at = e.ledger().timestamp();
        storage::write_batch(&e, &batch_id, &batch);
        storage::extend_instance_ttl(&e);
        events::batch_sealed(&e, &batch_id, &custodian);
    }

    pub fn get_batch(e: Env, batch_id: BytesN<32>) -> Batch {
        require_initialized(&e);
        if !storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }
        storage::read_batch(&e, &batch_id)
    }

    pub fn hop_count(e: Env, batch_id: BytesN<32>) -> u32 {
        require_initialized(&e);
        if !storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }
        storage::read_batch(&e, &batch_id).hops
    }

    /// Returns up to `limit` hops starting at `start`. Fewer entries near chain end.
    pub fn get_custody_chain(
        e: Env,
        batch_id: BytesN<32>,
        start: u32,
        limit: u32,
    ) -> Vec<CustodyHop> {
        require_initialized(&e);
        if limit > MAX_CHAIN_PAGE {
            panic_with_error!(&e, ContractError::LimitTooLarge);
        }
        if !storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let hops = storage::read_batch(&e, &batch_id).hops;
        let mut chain = Vec::new(&e);
        if start >= hops {
            return chain;
        }

        let end = start.saturating_add(limit).min(hops);
        let mut index = start;
        while index < end {
            chain.push_back(storage::read_hop(&e, &batch_id, index));
            index += 1;
        }
        chain
    }

    /// True only when sealed, certificate attached, and not revoked at call time.
    pub fn is_provenance_valid(e: Env, batch_id: BytesN<32>) -> bool {
        require_initialized(&e);
        if !storage::has_batch(&e, &batch_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let batch = storage::read_batch(&e, &batch_id);
        if batch.state != BatchState::Sealed {
            return false;
        }
        if !storage::has_certificate(&e, &batch_id) {
            return false;
        }

        let credential_id = storage::read_certificate(&e, &batch_id);
        !is_credential_revoked(&e, &batch.certifier, &credential_id)
    }

    pub fn version(e: Env) -> Symbol {
        Symbol::new(&e, VERSION_SYMBOL)
    }
}

fn require_initialized(e: &Env) {
    if !storage::has_admin(e) {
        panic_with_error!(e, CommonError::NotInitialized);
    }
}

fn is_credential_revoked(e: &Env, issuer: &Address, credential_id: &Bytes) -> bool {
    let registry = storage::read_revocation_registry(e);
    let client = RevocationRegistryClient::new(e, &registry);
    client.is_revoked(issuer, credential_id)
}
