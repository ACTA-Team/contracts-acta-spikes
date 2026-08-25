//! Contract entry points for vc-health-license-registry.

use crate::error::ContractError;
use crate::events;
use crate::storage;
use registry_core::{validate_max_bytes, CommonError, DEFAULT_MAX_BYTES};
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{
    contract, contractclient, contractimpl, contractmeta, contracttype, panic_with_error, Address,
    Bytes, BytesN, Env, Symbol,
};

const VERSION_SYMBOL: &str = "0_1_0";

// ---------------------------------------------------------------------------
// Cross-contract binding for vc-issuer-registry (mirrors only `is_issuer_allowed`).
// ---------------------------------------------------------------------------

#[contractclient(name = "IssuerRegistryClient")]
pub trait IssuerRegistryTrait {
    fn is_issuer_allowed(env: Env, issuer: Address) -> bool;
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LicenseStatus {
    Active,
    Suspended,
    Expired,
    Revoked,
}

#[contracttype]
#[derive(Clone)]
pub struct License {
    pub authority: Address,
    pub holder: Address,
    pub specialty: Symbol,
    pub jurisdiction: Symbol,
    pub issued_at: u64,
    pub expires_at: u64,
    pub suspended_until: u64,
    pub revoked: bool,
    pub metadata: Bytes,
}

contractmeta!(
    key = "Description",
    val = "VC Health License Registry: professional licenses with derived status",
);

#[contract]
pub struct VcHealthLicenseRegistryContract;

#[contractimpl]
impl VcHealthLicenseRegistryContract {
    /// Idempotent. Second call fails with `CommonError::AlreadyInitialized`.
    pub fn initialize(e: Env, admin: Address, issuer_registry: Address) {
        if storage::has_admin(&e) {
            panic_with_error!(&e, CommonError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&e, &admin);
        storage::write_issuer_registry(&e, &issuer_registry);
        storage::extend_instance_ttl(&e);
    }

    /// `sha256(xdr(authority) || xdr(holder) || xdr(specialty) || xdr(jurisdiction))`
    pub fn license_id(
        e: Env,
        authority: Address,
        holder: Address,
        specialty: Symbol,
        jurisdiction: Symbol,
    ) -> BytesN<32> {
        compute_license_id(&e, &authority, &holder, &specialty, &jurisdiction)
    }

    /// Requires `authority.require_auth()` and `is_issuer_allowed(authority) == true`.
    /// `expires_at` must be strictly greater than the current ledger timestamp.
    pub fn issue_license(
        e: Env,
        authority: Address,
        holder: Address,
        specialty: Symbol,
        jurisdiction: Symbol,
        expires_at: u64,
        metadata: Bytes,
    ) -> BytesN<32> {
        require_initialized(&e);
        authority.require_auth();
        storage::extend_instance_ttl(&e);

        if !is_authority_allowed(&e, &authority) {
            panic_with_error!(&e, ContractError::AuthorityNotAllowed);
        }

        validate_max_bytes(&e, &metadata, DEFAULT_MAX_BYTES);

        let now = e.ledger().timestamp();
        if expires_at <= now {
            panic_with_error!(&e, ContractError::ExpiryInPast);
        }

        let license_id = compute_license_id(&e, &authority, &holder, &specialty, &jurisdiction);

        if storage::has_license(&e, &license_id) {
            panic_with_error!(&e, CommonError::AlreadyExists);
        }

        let license = License {
            authority: authority.clone(),
            holder,
            specialty,
            jurisdiction,
            issued_at: now,
            expires_at,
            suspended_until: 0,
            revoked: false,
            metadata,
        };

        storage::write_license(&e, &license_id, &license);
        events::license_issued(&e, &license_id, &authority);
        license_id
    }

    /// `new_expires_at` must be strictly greater than `max(now, current expires_at)`.
    pub fn renew_license(e: Env, authority: Address, license_id: BytesN<32>, new_expires_at: u64) {
        require_initialized(&e);
        authority.require_auth();
        storage::extend_instance_ttl(&e);

        let mut license = load_license_for_authority(&e, &authority, &license_id);
        reject_if_revoked(&e, &license);

        let now = e.ledger().timestamp();
        let min_expires = now.max(license.expires_at);
        if new_expires_at <= min_expires {
            panic_with_error!(&e, ContractError::RenewalNotMonotonic);
        }

        license.expires_at = new_expires_at;
        storage::write_license(&e, &license_id, &license);
        events::license_renewed(&e, &license_id, &authority);
    }

    /// `until` must be strictly greater than now. Suspending a revoked license fails.
    pub fn suspend_license(
        e: Env,
        authority: Address,
        license_id: BytesN<32>,
        until: u64,
        reason: Symbol,
    ) {
        require_initialized(&e);
        authority.require_auth();
        storage::extend_instance_ttl(&e);

        let mut license = load_license_for_authority(&e, &authority, &license_id);
        reject_if_revoked(&e, &license);

        let now = e.ledger().timestamp();
        if until <= now {
            panic_with_error!(&e, ContractError::SuspensionInPast);
        }

        license.suspended_until = until;
        storage::write_license(&e, &license_id, &license);
        events::license_suspended(&e, &license_id, &authority, until, &reason);
    }

    /// Fails if the license is not currently suspended.
    pub fn lift_suspension(e: Env, authority: Address, license_id: BytesN<32>) {
        require_initialized(&e);
        authority.require_auth();
        storage::extend_instance_ttl(&e);

        let mut license = load_license_for_authority(&e, &authority, &license_id);
        reject_if_revoked(&e, &license);

        let now = e.ledger().timestamp();
        if license.suspended_until <= now {
            panic_with_error!(&e, ContractError::NotSuspended);
        }

        license.suspended_until = 0;
        storage::write_license(&e, &license_id, &license);
        events::suspension_lifted(&e, &license_id, &authority);
    }

    /// Terminal. A revoked license can never return to any other status.
    pub fn revoke_license(e: Env, authority: Address, license_id: BytesN<32>) {
        require_initialized(&e);
        authority.require_auth();
        storage::extend_instance_ttl(&e);

        let mut license = load_license_for_authority(&e, &authority, &license_id);
        reject_if_revoked(&e, &license);

        license.revoked = true;
        storage::write_license(&e, &license_id, &license);
        events::license_revoked(&e, &license_id, &authority);
    }

    pub fn get_license(e: Env, license_id: BytesN<32>) -> License {
        require_initialized(&e);
        storage::extend_instance_ttl(&e);

        if !storage::has_license(&e, &license_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }
        storage::read_license(&e, &license_id)
    }

    /// Pure function of (stored record, current ledger timestamp).
    pub fn license_status(e: Env, license_id: BytesN<32>) -> LicenseStatus {
        require_initialized(&e);
        storage::extend_instance_ttl(&e);

        if !storage::has_license(&e, &license_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }
        let license = storage::read_license(&e, &license_id);
        derive_status(&license, e.ledger().timestamp())
    }

    /// Returns true when `license_status == Active`.
    pub fn is_license_valid(e: Env, license_id: BytesN<32>) -> bool {
        Self::license_status(e, license_id) == LicenseStatus::Active
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

fn is_authority_allowed(e: &Env, authority: &Address) -> bool {
    let registry = storage::read_issuer_registry(e);
    let client = IssuerRegistryClient::new(e, &registry);
    client.is_issuer_allowed(authority)
}

fn load_license_for_authority(e: &Env, authority: &Address, license_id: &BytesN<32>) -> License {
    if !storage::has_license(e, license_id) {
        panic_with_error!(e, CommonError::NotFound);
    }
    let license = storage::read_license(e, license_id);
    if license.authority != *authority {
        panic_with_error!(e, CommonError::Unauthorized);
    }
    license
}

fn reject_if_revoked(e: &Env, license: &License) {
    if license.revoked {
        panic_with_error!(e, ContractError::LicenseRevoked);
    }
}

/// Precedence: revoked → expired → suspended → active.
fn derive_status(license: &License, now: u64) -> LicenseStatus {
    if license.revoked {
        LicenseStatus::Revoked
    } else if now >= license.expires_at {
        LicenseStatus::Expired
    } else if license.suspended_until > now {
        LicenseStatus::Suspended
    } else {
        LicenseStatus::Active
    }
}

/// Computes `sha256(xdr(authority) || xdr(holder) || xdr(specialty) || xdr(jurisdiction))`.
fn compute_license_id(
    e: &Env,
    authority: &Address,
    holder: &Address,
    specialty: &Symbol,
    jurisdiction: &Symbol,
) -> BytesN<32> {
    let mut preimage = Bytes::new(e);
    preimage.append(&authority.clone().to_xdr(e));
    preimage.append(&holder.clone().to_xdr(e));
    preimage.append(&specialty.clone().to_xdr(e));
    preimage.append(&jurisdiction.clone().to_xdr(e));
    e.crypto().sha256(&preimage).to_bytes()
}
