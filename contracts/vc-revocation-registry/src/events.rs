//! Contract events. Published on key state transitions for on-chain observability.

use soroban_sdk::{contractevent, Address, Bytes, Env};

/// Emitted when the contract is initialized.
#[contractevent]
pub struct Initialized {
    pub admin: Address,
}

/// Emitted when a credential is revoked.
#[contractevent]
pub struct CredentialRevoked {
    pub issuer: Address,
    pub credential_id: Bytes,
}

/// Emitted when a credential is unrevoked (removed from the revocation registry).
#[contractevent]
pub struct CredentialUnrevoked {
    pub issuer: Address,
    pub credential_id: Bytes,
}

pub fn initialized(e: &Env, admin: &Address) {
    Initialized {
        admin: admin.clone(),
    }
    .publish(e);
}

pub fn credential_revoked(e: &Env, issuer: &Address, credential_id: &Bytes) {
    CredentialRevoked {
        issuer: issuer.clone(),
        credential_id: credential_id.clone(),
    }
    .publish(e);
}

pub fn credential_unrevoked(e: &Env, issuer: &Address, credential_id: &Bytes) {
    CredentialUnrevoked {
        issuer: issuer.clone(),
        credential_id: credential_id.clone(),
    }
    .publish(e);
}
