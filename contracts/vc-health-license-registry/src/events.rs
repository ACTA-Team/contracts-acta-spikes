//! Contract events for vc-health-license-registry.

use soroban_sdk::{contractevent, Address, BytesN, Env, Symbol};

#[contractevent]
pub struct LicenseIssued {
    pub license_id: BytesN<32>,
    pub authority: Address,
}

#[contractevent]
pub struct LicenseRenewed {
    pub license_id: BytesN<32>,
    pub authority: Address,
}

#[contractevent]
pub struct LicenseSuspended {
    pub license_id: BytesN<32>,
    pub authority: Address,
    pub until: u64,
    pub reason: Symbol,
}

#[contractevent]
pub struct SuspensionLifted {
    pub license_id: BytesN<32>,
    pub authority: Address,
}

#[contractevent]
pub struct LicenseRevoked {
    pub license_id: BytesN<32>,
    pub authority: Address,
}

pub fn license_issued(e: &Env, license_id: &BytesN<32>, authority: &Address) {
    LicenseIssued {
        license_id: license_id.clone(),
        authority: authority.clone(),
    }
    .publish(e);
}

pub fn license_renewed(e: &Env, license_id: &BytesN<32>, authority: &Address) {
    LicenseRenewed {
        license_id: license_id.clone(),
        authority: authority.clone(),
    }
    .publish(e);
}

pub fn license_suspended(
    e: &Env,
    license_id: &BytesN<32>,
    authority: &Address,
    until: u64,
    reason: &Symbol,
) {
    LicenseSuspended {
        license_id: license_id.clone(),
        authority: authority.clone(),
        until,
        reason: reason.clone(),
    }
    .publish(e);
}

pub fn suspension_lifted(e: &Env, license_id: &BytesN<32>, authority: &Address) {
    SuspensionLifted {
        license_id: license_id.clone(),
        authority: authority.clone(),
    }
    .publish(e);
}

pub fn license_revoked(e: &Env, license_id: &BytesN<32>, authority: &Address) {
    LicenseRevoked {
        license_id: license_id.clone(),
        authority: authority.clone(),
    }
    .publish(e);
}
