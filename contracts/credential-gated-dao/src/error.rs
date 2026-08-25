//! Contract-specific error codes. Codes 1–9 are reserved for `registry_core::CommonError`.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// `vote` called at or after `closes_at`.
    VotingClosed = 10,
    /// `finalize` called before `closes_at`.
    VotingOpen = 11,
    /// This voter already voted on this proposal.
    AlreadyVoted = 12,
    /// Proposal state is not `Open`.
    AlreadyFinalized = 13,
    /// `get_schema_weight(schema_id) == 0`.
    SchemaNotWeighted = 14,
    /// `verify` returned an invalid result.
    CredentialNotValid = 15,
    /// `voting_period` outside the allowed range.
    InvalidVotingPeriod = 16,
    /// Checked tally addition overflowed.
    WeightOverflow = 17,
}
