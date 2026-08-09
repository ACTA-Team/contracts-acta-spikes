//! Contract events emitted on state changes.

use soroban_sdk::{contractevent, Address, Env};

#[contractevent]
pub struct Initialized {
    pub admin: Address,
    pub issuer_registry: Address,
    pub schema_registry: Address,
    pub revocation_registry: Address,
}

#[contractevent]
pub struct IssuerRegistryUpdated {
    pub new_address: Address,
}

#[contractevent]
pub struct SchemaRegistryUpdated {
    pub new_address: Address,
}

#[contractevent]
pub struct RevocationRegistryUpdated {
    pub new_address: Address,
}

pub fn initialized(
    e: &Env,
    admin: &Address,
    issuer_registry: &Address,
    schema_registry: &Address,
    revocation_registry: &Address,
) {
    Initialized {
        admin: admin.clone(),
        issuer_registry: issuer_registry.clone(),
        schema_registry: schema_registry.clone(),
        revocation_registry: revocation_registry.clone(),
    }
    .publish(e);
}

pub fn issuer_registry_updated(e: &Env, new_address: &Address) {
    IssuerRegistryUpdated {
        new_address: new_address.clone(),
    }
    .publish(e);
}

pub fn schema_registry_updated(e: &Env, new_address: &Address) {
    SchemaRegistryUpdated {
        new_address: new_address.clone(),
    }
    .publish(e);
}

pub fn revocation_registry_updated(e: &Env, new_address: &Address) {
    RevocationRegistryUpdated {
        new_address: new_address.clone(),
    }
    .publish(e);
}
