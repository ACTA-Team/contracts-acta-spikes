//! Contract events. Published on key state transitions for on-chain observability.

use soroban_sdk::{contractevent, Address, BytesN, Env};

#[contractevent]
pub struct Initialized {
    pub admin: Address,
}

#[contractevent]
pub struct Revoked {
    pub vc_id: BytesN<32>,
    pub issuer: Address,
}

#[contractevent]
pub struct Unrevoked {
    pub vc_id: BytesN<32>,
}

pub fn initialized(e: &Env, admin: &Address) {
    Initialized {
        admin: admin.clone(),
    }
    .publish(e);
}

pub fn revoked(e: &Env, vc_id: &BytesN<32>, issuer: &Address) {
    Revoked {
        vc_id: vc_id.clone(),
        issuer: issuer.clone(),
    }
    .publish(e);
}

pub fn unrevoked(e: &Env, vc_id: &BytesN<32>) {
    Unrevoked {
        vc_id: vc_id.clone(),
    }
    .publish(e);
}
