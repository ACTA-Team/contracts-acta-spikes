//! Contract entry points for credential-escrow.

#![allow(clippy::too_many_arguments)]

use crate::error::ContractError;
use crate::events;
use crate::storage;
use registry_core::CommonError;
use soroban_sdk::{
    contract, contractclient, contractimpl, contractmeta, contracttype, panic_with_error,
    token::TokenClient, Address, Bytes, BytesN, Env, Symbol,
};

const VERSION_SYMBOL: &str = "0_1_0";

// ---------------------------------------------------------------------------
// Cross-contract binding for vc-verifier (mirrors only fields we read).
// ---------------------------------------------------------------------------

#[contractclient(name = "VerifierClient")]
pub trait VerifierTrait {
    fn verify(
        env: Env,
        issuer: Address,
        schema_id: BytesN<32>,
        credential_id: Bytes,
    ) -> VerificationResult;
}

/// Minimal verification result mirrored from vc-verifier.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationResult {
    pub valid: bool,
    pub issuer_allowed: bool,
    pub schema_exists: bool,
    pub schema_deprecated: bool,
    pub revoked: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowState {
    Funded,
    Claimed,
    Refunded,
}

#[contracttype]
#[derive(Clone)]
pub struct Escrow {
    pub depositor: Address,
    pub beneficiary: Address,
    pub token: Address,
    pub amount: i128,
    pub schema_id: BytesN<32>,
    pub required_issuer: Address,
    pub deadline: u64,
    pub state: EscrowState,
    pub created_at: u64,
}

contractmeta!(
    key = "Description",
    val = "Credential Escrow: conditional token release against verified credentials",
);

#[contract]
pub struct CredentialEscrowContract;

#[contractimpl]
impl CredentialEscrowContract {
    /// Idempotent. Second call fails with `CommonError::AlreadyInitialized`.
    pub fn initialize(e: Env, admin: Address, verifier: Address) {
        if storage::has_admin(&e) {
            panic_with_error!(&e, CommonError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&e, &admin);
        storage::write_verifier(&e, &verifier);
        storage::write_next_escrow_id(&e, 1);
        storage::extend_instance_ttl(&e);
    }

    /// Transfers `amount` of `token` from the depositor into this contract before
    /// recording the escrow. Returns a strictly increasing id.
    pub fn create_escrow(
        e: Env,
        depositor: Address,
        beneficiary: Address,
        token: Address,
        amount: i128,
        schema_id: BytesN<32>,
        required_issuer: Address,
        deadline: u64,
    ) -> u64 {
        require_initialized(&e);
        depositor.require_auth();

        if amount <= 0 {
            panic_with_error!(&e, ContractError::InvalidAmount);
        }

        let now = e.ledger().timestamp();
        if deadline <= now {
            panic_with_error!(&e, ContractError::DeadlineInPast);
        }

        if beneficiary == depositor {
            panic_with_error!(&e, ContractError::SelfEscrow);
        }

        let contract_addr = e.current_contract_address();
        let token_client = TokenClient::new(&e, &token);
        token_client.transfer(&depositor, &contract_addr, &amount);

        let escrow_id = storage::read_next_escrow_id(&e);
        storage::write_next_escrow_id(&e, escrow_id + 1);

        let escrow = Escrow {
            depositor: depositor.clone(),
            beneficiary: beneficiary.clone(),
            token: token.clone(),
            amount,
            schema_id: schema_id.clone(),
            required_issuer: required_issuer.clone(),
            deadline,
            state: EscrowState::Funded,
            created_at: now,
        };

        storage::write_escrow(&e, escrow_id, &escrow);
        storage::extend_instance_ttl(&e);
        events::escrow_created(&e, escrow_id, &depositor, &beneficiary, &token, amount);

        escrow_id
    }

    /// Calls `verifier.verify` and transfers the full amount to the beneficiary
    /// when the credential is valid.
    pub fn claim(e: Env, escrow_id: u64, beneficiary: Address, credential_id: Bytes) {
        require_initialized(&e);
        beneficiary.require_auth();

        if !storage::has_escrow(&e, escrow_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let mut escrow = storage::read_escrow(&e, escrow_id);

        if escrow.beneficiary != beneficiary {
            panic_with_error!(&e, ContractError::NotBeneficiary);
        }

        if escrow.state != EscrowState::Funded {
            panic_with_error!(&e, ContractError::EscrowNotFunded);
        }

        let now = e.ledger().timestamp();
        if now >= escrow.deadline {
            panic_with_error!(&e, ContractError::DeadlinePassed);
        }

        let verifier_addr = storage::read_verifier(&e);
        let verifier = VerifierClient::new(&e, &verifier_addr);
        let result = verifier.verify(&escrow.required_issuer, &escrow.schema_id, &credential_id);
        if !result.valid {
            panic_with_error!(&e, ContractError::CredentialNotValid);
        }

        escrow.state = EscrowState::Claimed;
        storage::write_escrow(&e, escrow_id, &escrow);

        let contract_addr = e.current_contract_address();
        let token_client = TokenClient::new(&e, &escrow.token);
        token_client.transfer(&contract_addr, &beneficiary, &escrow.amount);

        storage::extend_instance_ttl(&e);
        events::escrow_claimed(
            &e,
            escrow_id,
            &beneficiary,
            &escrow.depositor,
            &escrow.token,
            escrow.amount,
        );
    }

    /// Returns the full amount to the depositor after the deadline.
    pub fn refund(e: Env, escrow_id: u64, depositor: Address) {
        require_initialized(&e);
        depositor.require_auth();

        if !storage::has_escrow(&e, escrow_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }

        let mut escrow = storage::read_escrow(&e, escrow_id);

        if escrow.depositor != depositor {
            panic_with_error!(&e, ContractError::NotDepositor);
        }

        if escrow.state != EscrowState::Funded {
            panic_with_error!(&e, ContractError::EscrowNotFunded);
        }

        let now = e.ledger().timestamp();
        if now < escrow.deadline {
            panic_with_error!(&e, ContractError::DeadlineNotReached);
        }

        escrow.state = EscrowState::Refunded;
        storage::write_escrow(&e, escrow_id, &escrow);

        let contract_addr = e.current_contract_address();
        let token_client = TokenClient::new(&e, &escrow.token);
        token_client.transfer(&contract_addr, &depositor, &escrow.amount);

        storage::extend_instance_ttl(&e);
        events::escrow_refunded(
            &e,
            escrow_id,
            &depositor,
            &escrow.beneficiary,
            &escrow.token,
            escrow.amount,
        );
    }

    pub fn get_escrow(e: Env, escrow_id: u64) -> Escrow {
        require_initialized(&e);
        if !storage::has_escrow(&e, escrow_id) {
            panic_with_error!(&e, CommonError::NotFound);
        }
        storage::read_escrow(&e, escrow_id)
    }

    pub fn version(e: Env) -> Symbol {
        Symbol::new(&e, VERSION_SYMBOL)
    }
}

fn require_initialized(e: &Env) {
    if !storage::has_admin(e) {
        panic_with_error!(e, CommonError::NotInitialized);
    }
}
