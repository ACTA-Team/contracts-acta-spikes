//! Contract events. Published on key state transitions for on-chain observability.

use soroban_sdk::{contractevent, Address, BytesN, Env};

#[contractevent]
pub struct DidRegistered {
    pub did: BytesN<32>,
    pub controller: Address,
}

#[contractevent]
pub struct DidUpdated {
    pub did: BytesN<32>,
    pub controller: Address,
    pub version: u32,
}

#[contractevent]
pub struct DidDeactivated {
    pub did: BytesN<32>,
    pub controller: Address,
}

pub fn did_registered(e: &Env, did: &BytesN<32>, controller: &Address) {
    DidRegistered {
        did: did.clone(),
        controller: controller.clone(),
    }
    .publish(e);
}

pub fn did_updated(e: &Env, did: &BytesN<32>, controller: &Address, version: u32) {
    DidUpdated {
        did: did.clone(),
        controller: controller.clone(),
        version,
    }
    .publish(e);
}

pub fn did_deactivated(e: &Env, did: &BytesN<32>, controller: &Address) {
    DidDeactivated {
        did: did.clone(),
        controller: controller.clone(),
    }
    .publish(e);
}
