//! Contract events for credential-escrow.

use soroban_sdk::{contractevent, Address, Env};

#[contractevent]
pub struct EscrowCreated {
    pub escrow_id: u64,
    pub depositor: Address,
    pub beneficiary: Address,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
pub struct EscrowClaimed {
    pub escrow_id: u64,
    pub beneficiary: Address,
    pub depositor: Address,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
pub struct EscrowRefunded {
    pub escrow_id: u64,
    pub depositor: Address,
    pub beneficiary: Address,
    pub token: Address,
    pub amount: i128,
}

pub fn escrow_created(
    e: &Env,
    escrow_id: u64,
    depositor: &Address,
    beneficiary: &Address,
    token: &Address,
    amount: i128,
) {
    EscrowCreated {
        escrow_id,
        depositor: depositor.clone(),
        beneficiary: beneficiary.clone(),
        token: token.clone(),
        amount,
    }
    .publish(e);
}

pub fn escrow_claimed(
    e: &Env,
    escrow_id: u64,
    beneficiary: &Address,
    depositor: &Address,
    token: &Address,
    amount: i128,
) {
    EscrowClaimed {
        escrow_id,
        beneficiary: beneficiary.clone(),
        depositor: depositor.clone(),
        token: token.clone(),
        amount,
    }
    .publish(e);
}

pub fn escrow_refunded(
    e: &Env,
    escrow_id: u64,
    depositor: &Address,
    beneficiary: &Address,
    token: &Address,
    amount: i128,
) {
    EscrowRefunded {
        escrow_id,
        depositor: depositor.clone(),
        beneficiary: beneficiary.clone(),
        token: token.clone(),
        amount,
    }
    .publish(e);
}
