//! Contract events. Published on key state transitions for on-chain observability.

use soroban_sdk::{contractevent, Address, Bytes, Env};

#[contractevent]
pub struct Initialized {
    pub admin: Address,
}

#[contractevent]
pub struct SchemaRegistered {
    pub id: Bytes,
    pub author: Address,
}

#[contractevent]
pub struct SchemaDeprecated {
    pub id: Bytes,
}

#[contractevent]
pub struct SchemaUriUpdated {
    pub id: Bytes,
}

pub fn initialized(e: &Env, admin: &Address) {
    Initialized {
        admin: admin.clone(),
    }
    .publish(e);
}

pub fn schema_registered(e: &Env, id: &Bytes, author: &Address) {
    SchemaRegistered {
        id: id.clone(),
        author: author.clone(),
    }
    .publish(e);
}

pub fn schema_deprecated(e: &Env, id: &Bytes) {
    SchemaDeprecated { id: id.clone() }.publish(e);
}

pub fn schema_uri_updated(e: &Env, id: &Bytes) {
    SchemaUriUpdated { id: id.clone() }.publish(e);
}
