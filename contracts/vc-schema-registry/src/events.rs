//! Contract events. Published on every state-changing call.

use soroban_sdk::{contractevent, Address, BytesN, Env};

#[contractevent]
pub struct Initialized {
    pub admin: Address,
}

#[contractevent]
pub struct SchemaRegistered {
    pub schema_id: BytesN<32>,
    pub author: Address,
}

#[contractevent]
pub struct SchemaDeprecated {
    pub schema_id: BytesN<32>,
}

#[contractevent]
pub struct SchemaUriUpdated {
    pub schema_id: BytesN<32>,
}

pub fn initialized(e: &Env, admin: &Address) {
    Initialized { admin: admin.clone() }.publish(e);
}

pub fn schema_registered(e: &Env, schema_id: &BytesN<32>, author: &Address) {
    SchemaRegistered {
        schema_id: schema_id.clone(),
        author: author.clone(),
    }
    .publish(e);
}

pub fn schema_deprecated(e: &Env, schema_id: &BytesN<32>) {
    SchemaDeprecated { schema_id: schema_id.clone() }.publish(e);
}

pub fn schema_uri_updated(e: &Env, schema_id: &BytesN<32>) {
    SchemaUriUpdated { schema_id: schema_id.clone() }.publish(e);
}
