//! Property-based tests for vc-health-license-registry invariants.
//!
//! Covers invariants 1, 2, 3, and 5 from the issue:
//! 1. Revocation is monotonic.
//! 2. Expiry never decreases.
//! 3. Status is a pure function of (record, now).
//! 5. ID determinism and collision resistance.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Bytes, Env, Symbol};

use vc_issuer_registry_contract::contract::{
    VcIssuerRegistryContract, VcIssuerRegistryContractClient,
};

use crate::contract::{
    LicenseStatus, VcHealthLicenseRegistryContract, VcHealthLicenseRegistryContractClient,
};

const BASE_TS: u64 = 1_700_000_000;

struct PropEnv {
    e: Env,
    authority: Address,
    holder: Address,
    license: VcHealthLicenseRegistryContractClient<'static>,
    license_id: soroban_sdk::BytesN<32>,
}

fn setup_prop() -> PropEnv {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.timestamp = BASE_TS);

    let admin = Address::generate(&e);
    let authority = Address::generate(&e);
    let holder = Address::generate(&e);

    let issuer_reg_id = e.register(VcIssuerRegistryContract, ());
    let license_reg_id = e.register(VcHealthLicenseRegistryContract, ());

    let issuer_client = VcIssuerRegistryContractClient::new(&e, &issuer_reg_id);
    let license = VcHealthLicenseRegistryContractClient::new(&e, &license_reg_id);

    issuer_client.initialize(&admin);
    license.initialize(&admin, &issuer_reg_id);
    issuer_client.add_issuer(&authority, &None, &None, &None);

    let specialty = Symbol::new(&e, "PropSpec");
    let jurisdiction = Symbol::new(&e, "PropJuris");
    let license_id = license.issue_license(
        &authority,
        &holder,
        &specialty,
        &jurisdiction,
        &(BASE_TS + 86_400),
        &Bytes::from_slice(&e, b"prop-meta"),
    );

    PropEnv {
        e,
        authority,
        holder,
        license,
        license_id,
    }
}

proptest! {
    #[test]
    fn prop_revocation_is_monotonic(ops in prop::collection::vec(any::<u8>(), 1..80)) {
        let ctx = setup_prop();
        let mut revoked = false;

        for (idx, byte) in ops.iter().enumerate() {
            if revoked {
                assert_eq!(
                    ctx.license.license_status(&ctx.license_id),
                    LicenseStatus::Revoked
                );
            }

            match byte % 5 {
                0 => {
                    let bump = u64::from(*byte % 64) + 1;
                    let record = ctx.license.get_license(&ctx.license_id);
                    let new_expires = record.expires_at.saturating_add(bump);
                    let _ = ctx.license.try_renew_license(
                        &ctx.authority,
                        &ctx.license_id,
                        &new_expires,
                    );
                }
                1 => {
                    let now = ctx.e.ledger().timestamp();
                    let until = now.saturating_add(u64::from(*byte % 32) + 1);
                    let _ = ctx.license.try_suspend_license(
                        &ctx.authority,
                        &ctx.license_id,
                        &until,
                        &Symbol::new(&ctx.e, "PropReason"),
                    );
                }
                2 => {
                    let _ = ctx.license.try_lift_suspension(&ctx.authority, &ctx.license_id);
                }
                3 => {
                    if ctx.license.try_revoke_license(&ctx.authority, &ctx.license_id).is_ok() {
                        revoked = true;
                    }
                }
                _ => {
                    let advance = u64::from(*byte % 16) + 1;
                    ctx.e.ledger().with_mut(|l| {
                        l.timestamp = l.timestamp.saturating_add(advance);
                    });
                }
            }

            if idx % 7 == 0 && !revoked {
                let record = ctx.license.get_license(&ctx.license_id);
                if record.revoked {
                    revoked = true;
                }
            }
        }

        if revoked {
            assert_eq!(
                ctx.license.license_status(&ctx.license_id),
                LicenseStatus::Revoked
            );
        }
    }

    #[test]
    fn prop_expiry_never_decreases(ops in prop::collection::vec(any::<u8>(), 1..80)) {
        let ctx = setup_prop();
        let mut last_expires = ctx.license.get_license(&ctx.license_id).expires_at;

        for byte in ops {
            if byte % 4 == 0 {
                let bump = u64::from(byte % 64) + 1;
                let new_expires = last_expires.saturating_add(bump);
                if ctx
                    .license
                    .try_renew_license(&ctx.authority, &ctx.license_id, &new_expires)
                    .is_ok()
                {
                    last_expires = new_expires;
                }
            } else if byte % 4 == 1 {
                let now = ctx.e.ledger().timestamp();
                let until = now.saturating_add(u64::from(byte % 32) + 1);
                let _ = ctx.license.try_suspend_license(
                    &ctx.authority,
                    &ctx.license_id,
                    &until,
                    &Symbol::new(&ctx.e, "PropReason"),
                );
            } else if byte % 4 == 2 {
                let _ = ctx.license.try_lift_suspension(&ctx.authority, &ctx.license_id);
            } else {
                ctx.e.ledger().with_mut(|l| {
                    l.timestamp = l.timestamp.saturating_add(u64::from(byte % 8) + 1);
                });
            }

            let current = ctx.license.get_license(&ctx.license_id).expires_at;
            prop_assert!(current >= last_expires);
            last_expires = current;
        }
    }

    #[test]
    fn prop_status_is_pure_function_of_record_and_now(_seed in any::<u8>()) {
        let ctx = setup_prop();
        let record_before = ctx.license.get_license(&ctx.license_id);
        let status_a = ctx.license.license_status(&ctx.license_id);
        let status_b = ctx.license.license_status(&ctx.license_id);
        let record_after = ctx.license.get_license(&ctx.license_id);

        prop_assert_eq!(status_a, status_b);
        prop_assert_eq!(record_before.expires_at, record_after.expires_at);
        prop_assert_eq!(record_before.revoked, record_after.revoked);
        prop_assert_eq!(record_before.suspended_until, record_after.suspended_until);
    }

    #[test]
    fn prop_license_id_deterministic_and_distinct(
        holder_suffix in 0u8..200,
        specialty_suffix in 0u8..200,
    ) {
        let ctx = setup_prop();
        let specialty_a = Symbol::new(&ctx.e, "SpecA");
        let specialty_b = Symbol::new(&ctx.e, "SpecB");
        let jurisdiction = Symbol::new(&ctx.e, "Juris");

        let holder_a = ctx.holder.clone();
        let mut holder_b = Address::generate(&ctx.e);
        if holder_suffix % 2 == 0 {
            holder_b = ctx.holder.clone();
        }

        let id_a1 = ctx
            .license
            .license_id(&ctx.authority, &holder_a, &specialty_a, &jurisdiction);
        let id_a2 = ctx
            .license
            .license_id(&ctx.authority, &holder_a, &specialty_a, &jurisdiction);
        prop_assert_eq!(id_a1.clone(), id_a2);

        let spec_for_b = if specialty_suffix % 2 == 0 {
            specialty_a.clone()
        } else {
            specialty_b
        };
        let id_b = ctx
            .license
            .license_id(&ctx.authority, &holder_b, &spec_for_b, &jurisdiction);

        if holder_b != holder_a || spec_for_b != specialty_a {
            prop_assert_ne!(id_a1, id_b);
        }
    }
}
