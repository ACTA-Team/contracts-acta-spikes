//! Contract events for vc-supply-chain-provenance.

use soroban_sdk::{contractevent, Address, Bytes, BytesN, Env, Symbol};

#[contractevent]
pub struct BatchRegistered {
    pub batch_id: BytesN<32>,
    pub certifier: Address,
    pub product: Symbol,
    pub origin: Symbol,
}

#[contractevent]
pub struct CertificateAttached {
    pub batch_id: BytesN<32>,
    pub certifier: Address,
    pub credential_id: Bytes,
}

#[contractevent]
pub struct CustodyTransferred {
    pub batch_id: BytesN<32>,
    pub from: Address,
    pub to: Address,
    pub hop_index: u32,
}

#[contractevent]
pub struct BatchSealed {
    pub batch_id: BytesN<32>,
    pub custodian: Address,
}

pub fn batch_registered(
    e: &Env,
    batch_id: &BytesN<32>,
    certifier: &Address,
    product: &Symbol,
    origin: &Symbol,
) {
    BatchRegistered {
        batch_id: batch_id.clone(),
        certifier: certifier.clone(),
        product: product.clone(),
        origin: origin.clone(),
    }
    .publish(e);
}

pub fn certificate_attached(
    e: &Env,
    batch_id: &BytesN<32>,
    certifier: &Address,
    credential_id: &Bytes,
) {
    CertificateAttached {
        batch_id: batch_id.clone(),
        certifier: certifier.clone(),
        credential_id: credential_id.clone(),
    }
    .publish(e);
}

pub fn custody_transferred(
    e: &Env,
    batch_id: &BytesN<32>,
    from: &Address,
    to: &Address,
    hop_index: u32,
) {
    CustodyTransferred {
        batch_id: batch_id.clone(),
        from: from.clone(),
        to: to.clone(),
        hop_index,
    }
    .publish(e);
}

pub fn batch_sealed(e: &Env, batch_id: &BytesN<32>, custodian: &Address) {
    BatchSealed {
        batch_id: batch_id.clone(),
        custodian: custodian.clone(),
    }
    .publish(e);
}
