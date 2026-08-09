//! Contract events. Published on key state transitions for on-chain observability.

use soroban_sdk::{contractevent, Address, Env, Symbol};

/// Emitted when a new status list is created.
#[contractevent]
pub struct ListCreated {
    pub issuer: Address,
    pub list_id: Symbol,
    pub size: u32,
}

/// Emitted when a single bit is flipped in a status list.
#[contractevent]
pub struct StatusChanged {
    pub issuer: Address,
    pub list_id: Symbol,
    pub index: u32,
    pub revoked: bool,
}

/// Emitted when a batch of bits is flipped in a status list.
#[contractevent]
pub struct StatusBatchChanged {
    pub issuer: Address,
    pub list_id: Symbol,
    pub count: u32,
    pub revoked: bool,
}

pub fn list_created(e: &Env, issuer: &Address, list_id: &Symbol, size: u32) {
    ListCreated {
        issuer: issuer.clone(),
        list_id: list_id.clone(),
        size,
    }
    .publish(e);
}

pub fn status_changed(e: &Env, issuer: &Address, list_id: &Symbol, index: u32, revoked: bool) {
    StatusChanged {
        issuer: issuer.clone(),
        list_id: list_id.clone(),
        index,
        revoked,
    }
    .publish(e);
}

pub fn status_batch_changed(
    e: &Env,
    issuer: &Address,
    list_id: &Symbol,
    count: u32,
    revoked: bool,
) {
    StatusBatchChanged {
        issuer: issuer.clone(),
        list_id: list_id.clone(),
        count,
        revoked,
    }
    .publish(e);
}
