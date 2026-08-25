//! Property-based tests for vc-supply-chain-provenance custody invariants.
//!
//! Covers invariants 1–5 from the issue:
//! 1. The custodian always matches the chain.
//! 2. Hops are append-only.
//! 3. Hop timestamps are non-decreasing.
//! 4. Sealing is terminal.
//! 5. `hop_count` is monotonically increasing and equals successful transfers.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN, Env, Symbol,
};

use vc_revocation_registry_contract::contract::{
    VcRevocationRegistryContract, VcRevocationRegistryContractClient,
};

use crate::contract::{
    BatchState, VcSupplyChainProvenanceContract, VcSupplyChainProvenanceContractClient,
};

struct PropEnv {
    e: Env,
    certifier: Address,
    provenance: VcSupplyChainProvenanceContractClient<'static>,
    batch_id: BytesN<32>,
}

fn setup_prop() -> PropEnv {
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

    let mut bytes = [0u8; 32];
    bytes[0] = 42;
    let batch_id = BytesN::from_array(&e, &bytes);

    provenance.register_batch(
        &certifier,
        &batch_id,
        &Symbol::new(&e, "PropProduct"),
        &Symbol::new(&e, "PropOrigin"),
        &Bytes::from_slice(&e, b"meta"),
    );

    PropEnv {
        e,
        certifier,
        provenance,
        batch_id,
    }
}

fn expected_custodian(ctx: &PropEnv) -> Address {
    let hops = ctx.provenance.hop_count(&ctx.batch_id);
    if hops == 0 {
        ctx.certifier.clone()
    } else {
        let chain = ctx
            .provenance
            .get_custody_chain(&ctx.batch_id, &(hops - 1), &1);
        chain.get(0).unwrap().to
    }
}

proptest! {
    #[test]
    fn prop_custodian_matches_chain(
        ops in prop::collection::vec(any::<u8>(), 0..=30),
        seal in any::<bool>(),
    ) {
        let ctx = setup_prop();
        let mut custodian = ctx.certifier.clone();
        let mut successful_transfers = 0u32;
        let mut sealed = false;

        for (idx, byte) in ops.iter().enumerate() {
            if idx % 5 == 0 {
                ctx.e.ledger().with_mut(|l| l.timestamp = l.timestamp.saturating_add(u64::from(*byte)));
            }

            let batch = ctx.provenance.get_batch(&ctx.batch_id);
            if batch.state == BatchState::Sealed {
                sealed = true;
                break;
            }

            if *byte % 3 == 0 {
                let cred = Bytes::from_slice(&ctx.e, &[*byte, idx as u8]);
                let _ = ctx.provenance.try_attach_certificate(
                    &ctx.certifier,
                    &ctx.batch_id,
                    &cred,
                );
            } else {
                let next = Address::generate(&ctx.e);
                if ctx.provenance
                    .try_transfer_custody(&ctx.batch_id, &custodian, &next)
                    .is_ok()
                {
                    custodian = next;
                    successful_transfers += 1;
                }
            }
        }

        let batch = ctx.provenance.get_batch(&ctx.batch_id);
        prop_assert_eq!(batch.custodian.clone(), expected_custodian(&ctx));
        prop_assert_eq!(ctx.provenance.hop_count(&ctx.batch_id), successful_transfers);

        if !sealed && batch.state == BatchState::InTransit {
            let cred = Bytes::from_slice(&ctx.e, b"prop-seal-cert");
            let _ = ctx.provenance.try_attach_certificate(
                &ctx.certifier,
                &ctx.batch_id,
                &cred,
            );
            if seal {
                let custodian = batch.custodian.clone();
                let _ = ctx.provenance.try_seal_batch(&ctx.batch_id, &custodian);
            }
        }

        let final_batch = ctx.provenance.get_batch(&ctx.batch_id);
        if final_batch.state == BatchState::Sealed {
            let next = Address::generate(&ctx.e);
            prop_assert!(ctx.provenance
                .try_transfer_custody(&ctx.batch_id, &final_batch.custodian, &next)
                .is_err());
            let cred = Bytes::from_slice(&ctx.e, b"after-seal");
            prop_assert!(ctx.provenance
                .try_attach_certificate(&ctx.certifier, &ctx.batch_id, &cred)
                .is_err());
        }
    }

    #[test]
    fn prop_hops_are_immutable(
        ops in prop::collection::vec(any::<u8>(), 1..=20),
    ) {
        let ctx = setup_prop();
        let mut custodian = ctx.certifier.clone();

        for byte in ops.iter() {
            let next = Address::generate(&ctx.e);
            if ctx.provenance
                .try_transfer_custody(&ctx.batch_id, &custodian, &next)
                .is_ok()
            {
                custodian = next;
                ctx.e.ledger().with_mut(|l| l.timestamp = l.timestamp.saturating_add(u64::from(*byte)));
            }
        }

        let hops = ctx.provenance.hop_count(&ctx.batch_id);
        if hops == 0 {
            return Ok(());
        }

        let snapshot_index = (ops[0] as u32) % hops;
        let snapshot = ctx.provenance
            .get_custody_chain(&ctx.batch_id, &snapshot_index, &1)
            .get(0)
            .unwrap();

        for byte in ops.iter().skip(1) {
            let next = Address::generate(&ctx.e);
            if ctx.provenance
                .try_transfer_custody(&ctx.batch_id, &custodian, &next)
                .is_ok()
            {
                custodian = next;
                ctx.e.ledger().with_mut(|l| l.timestamp = l.timestamp.saturating_add(u64::from(*byte)));
            }
        }

        let replay = ctx.provenance
            .get_custody_chain(&ctx.batch_id, &snapshot_index, &1)
            .get(0)
            .unwrap();
        prop_assert_eq!(snapshot.from, replay.from);
        prop_assert_eq!(snapshot.to, replay.to);
        prop_assert_eq!(snapshot.at, replay.at);
    }

    #[test]
    fn prop_hop_timestamps_non_decreasing(
        count in 1u32..=20,
    ) {
        let ctx = setup_prop();
        let mut custodian = ctx.certifier.clone();

        for i in 0..count {
            ctx.e.ledger().with_mut(|l| l.timestamp = l.timestamp.saturating_add(u64::from(i + 1)));
            let next = Address::generate(&ctx.e);
            ctx.provenance.transfer_custody(&ctx.batch_id, &custodian, &next);
            custodian = next;
        }

        let hops = ctx.provenance.hop_count(&ctx.batch_id);
        let chain = ctx.provenance.get_custody_chain(&ctx.batch_id, &0, &hops);
        let mut prev_at = 0u64;
        for i in 0..chain.len() {
            let hop = chain.get(i).unwrap();
            prop_assert!(hop.at >= prev_at);
            prev_at = hop.at;
        }
    }
}
