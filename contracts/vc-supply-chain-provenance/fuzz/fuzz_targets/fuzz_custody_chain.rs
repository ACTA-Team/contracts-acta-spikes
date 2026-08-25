//! Fuzz target for custody chain: custodian matches last hop and sealing is terminal.

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN, Env, Symbol,
};

use vc_revocation_registry_contract::contract::{
    VcRevocationRegistryContract, VcRevocationRegistryContractClient,
};
use vc_supply_chain_provenance_contract::contract::{
    BatchState, VcSupplyChainProvenanceContract, VcSupplyChainProvenanceContractClient,
};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = 1_700_000_000);

    let admin = Address::generate(&e);
    let certifier = Address::generate(&e);

    let revocation_reg_id = e.register(VcRevocationRegistryContract, ());
    let provenance_id = e.register(VcSupplyChainProvenanceContract, ());

    let revocation_client = VcRevocationRegistryContractClient::new(&e, &revocation_reg_id);
    let provenance = VcSupplyChainProvenanceContractClient::new(&e, &provenance_id);

    revocation_client.initialize(&admin);
    provenance.initialize(&admin, &revocation_reg_id);

    let mut batch_bytes = [0u8; 32];
    batch_bytes[0] = data[0];
    let batch_id = BytesN::from_array(&e, &batch_bytes);

    provenance.register_batch(
        &certifier,
        &batch_id,
        &Symbol::new(&e, "FuzzProduct"),
        &Symbol::new(&e, "FuzzOrigin"),
        &Bytes::from_slice(&e, b"fuzz-meta"),
    );

    let mut custodian = certifier.clone();
    let mut sealed = false;

    for (idx, byte) in data.iter().enumerate().skip(1) {
        if idx % 6 == 0 {
            e.ledger().with_mut(|l| l.timestamp = l.timestamp.saturating_add(u64::from(*byte)));
        }

        let batch = provenance.get_batch(&batch_id);
        if batch.state == BatchState::Sealed {
            sealed = true;
            break;
        }

        match byte % 4 {
            0 => {
                let cred = Bytes::from_slice(&e, &[*byte, idx as u8]);
                let _ = provenance.try_attach_certificate(&certifier, &batch_id, &cred);
            }
            1 => {
                if byte % 8 == 1 {
                    let cred = Bytes::from_slice(&e, b"fuzz-seal-cert");
                    let _ = provenance.try_attach_certificate(&certifier, &batch_id, &cred);
                    if provenance.try_seal_batch(&batch_id, &custodian).is_ok() {
                        sealed = true;
                        break;
                    }
                }
            }
            _ => {
                let next = Address::generate(&e);
                if provenance
                    .try_transfer_custody(&batch_id, &custodian, &next)
                    .is_ok()
                {
                    custodian = next;
                }
            }
        }
    }

    let batch = provenance.get_batch(&batch_id);
    let hops = provenance.hop_count(&batch_id);
    if hops > 0 {
        let last_hop = provenance
            .get_custody_chain(&batch_id, &(hops - 1), &1)
            .get(0)
            .unwrap();
        assert_eq!(batch.custodian, last_hop.to);
    } else {
        assert_eq!(batch.custodian, certifier);
    }

    if sealed || batch.state == BatchState::Sealed {
        let next = Address::generate(&e);
        assert!(provenance
            .try_transfer_custody(&batch_id, &batch.custodian, &next)
            .is_err());
        let cred = Bytes::from_slice(&e, b"post-seal");
        assert!(provenance
            .try_attach_certificate(&certifier, &batch_id, &cred)
            .is_err());
    }
});
