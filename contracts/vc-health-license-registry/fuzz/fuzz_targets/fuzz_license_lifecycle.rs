//! Fuzz target for license lifecycle: revocation is never undone and expiry never decreases.

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Bytes, Env, Symbol,
};

use vc_health_license_registry_contract::contract::{
    LicenseStatus, VcHealthLicenseRegistryContract, VcHealthLicenseRegistryContractClient,
};
use vc_issuer_registry_contract::contract::{
    VcIssuerRegistryContract, VcIssuerRegistryContractClient,
};

const BASE_TS: u64 = 1_700_000_000;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

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

    let specialty = Symbol::new(&e, "FuzzSpec");
    let jurisdiction = Symbol::new(&e, "FuzzJuris");
    let expires_at = BASE_TS + u64::from(data[0] % 64) + 1;

    let license_id = license.issue_license(
        &authority,
        &holder,
        &specialty,
        &jurisdiction,
        &expires_at,
        &Bytes::from_slice(&e, b"fuzz-meta"),
    );

    let mut last_expires = license.get_license(&license_id).expires_at;
    let mut ever_revoked = false;

    for (idx, byte) in data.iter().enumerate().skip(1) {
        if ever_revoked {
            assert_eq!(
                license.license_status(&license_id),
                LicenseStatus::Revoked
            );
        }

        match byte % 6 {
            0 => {
                let bump = u64::from(*byte % 32) + 1;
                let new_expires = last_expires.saturating_add(bump);
                if license
                    .try_renew_license(&authority, &license_id, &new_expires)
                    .is_ok()
                {
                    last_expires = new_expires;
                }
            }
            1 => {
                let now = e.ledger().timestamp();
                let until = now.saturating_add(u64::from(*byte % 16) + 1);
                let _ = license.try_suspend_license(
                    &authority,
                    &license_id,
                    &until,
                    &Symbol::new(&e, "FuzzReason"),
                );
            }
            2 => {
                let _ = license.try_lift_suspension(&authority, &license_id);
            }
            3 => {
                if license
                    .try_revoke_license(&authority, &license_id)
                    .is_ok()
                {
                    ever_revoked = true;
                }
            }
            4 => {
                e.ledger().with_mut(|l| {
                    l.timestamp = l.timestamp.saturating_add(u64::from(*byte % 8) + 1);
                });
            }
            _ => {
                let record = license.get_license(&license_id);
                if record.revoked {
                    ever_revoked = true;
                }
            }
        }

        let current = license.get_license(&license_id).expires_at;
        assert!(current >= last_expires);
        last_expires = current;

        if idx % 11 == 0 {
            let record = license.get_license(&license_id);
            if record.revoked {
                ever_revoked = true;
            }
        }
    }
});
