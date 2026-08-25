# credential-gated-dao

`credential-gated-dao` is a Soroban governance contract where voting power comes from verifiable credentials, not token balances. An admin assigns a voting weight per credential schema; voters prove they hold a live, unrevoked credential via a cross-contract call to `vc-verifier`, and their vote weight is fixed at cast time.

## Purpose

Token-weighted governance rewards capital. For professional bodies, standards consortia, or grant committees, the relevant question is qualification: *what are you credentialed to vote on?* This contract implements that model with bounded proposal lifecycles, one vote per voter, and permissionless finalization.

## Storage layout

| Key | Storage type | Description |
| --- | --- | --- |
| `Admin` (registry-core) | Instance | Contract admin |
| `Verifier` | Instance | `vc-verifier` contract address |
| `QuorumWeight` | Instance | Minimum total vote weight for passage |
| `NextProposalId` | Instance | Strictly increasing proposal counter |
| `Proposal(u64)` | Persistent | Proposal record and tallies |
| `SchemaWeight(BytesN<32>)` | Persistent | Voting weight per schema (`0` = no vote rights) |
| `Voted(u64, Address)` | Persistent | One entry per `(proposal, voter)` pair |

Voter tracking uses **one persistent key per `(proposal, voter)`**. The proposal record does not store a voter list, avoiding unbounded rewrites on every vote.

## Weight frozen at vote time

When a voter calls `vote`, the contract reads `get_schema_weight(schema_id)` and adds that weight to `yes_weight` or `no_weight`. `finalize` never re-reads schema weights. If the admin later changes a schema weight, existing proposal tallies are unchanged. This keeps governance outcomes stable and auditable.

## Quorum rule

After `closes_at`, anyone may call `finalize`. A proposal **passes** when:

1. `(yes_weight + no_weight) >= quorum_weight`, and
2. `yes_weight > no_weight`

Otherwise it is **rejected**. A tie rejects the proposal.

## Public API

### Initialization

- `initialize(env, admin: Address, verifier: Address, quorum_weight: u64)`
  - One-time setup. Stores admin, verifier, and quorum threshold.
  - Second call panics with `CommonError::AlreadyInitialized` (code 1).

### Admin

- `set_schema_weight(env, schema_id: BytesN<32>, weight: u32)`
  - Admin only. `weight == 0` removes voting rights for that schema.
  - Non-admin callers panic with `CommonError::Unauthorized` (code 5).

- `get_schema_weight(env, schema_id: BytesN<32>) -> u32`
  - Returns configured weight, or `0` if unset.

### Proposals

- `create_proposal(env, proposer: Address, metadata: Bytes, voting_period: u64) -> u64`
  - `proposer.require_auth()`.
  - `voting_period` must be in `[MIN_VOTING_PERIOD, MAX_VOTING_PERIOD]` (1 hour – 30 days).
  - `metadata` bounded by `registry_core::DEFAULT_MAX_BYTES` (256).
  - Returns a strictly increasing proposal id.

- `get_proposal(env, proposal_id: u64) -> Proposal`
  - Unknown id panics with `CommonError::NotFound` (code 2).

### Voting

- `vote(env, proposal_id, voter, issuer, credential_id, schema_id, support)`
  - `voter.require_auth()`.
  - Requires `Open` state and `now < closes_at`.
  - One vote per voter per proposal.
  - Calls `verifier.verify(issuer, schema_id, credential_id)`; proceeds only when `valid == true`.
  - Applies `get_schema_weight(schema_id)` at call time; weight must be `> 0`.

- `has_voted(env, proposal_id, voter) -> bool`

### Finalization

- `finalize(env, proposal_id)`
  - Permissionless after the voting window closes.
  - Single terminal transition from `Open` to `Passed` or `Rejected`.

### Metadata

- `version(env) -> Symbol`
  - Returns `"0_1_0"`.

## Error codes

Shared codes (`registry_core::CommonError`):

| Code | Variant | When |
| --- | --- | --- |
| 1 | AlreadyInitialized | Second `initialize` |
| 2 | NotFound | Unknown `proposal_id` |
| 5 | Unauthorized | Non-admin `set_schema_weight` |
| 6 | InvalidInput | Oversized `metadata` |

Contract-specific codes:

| Code | Variant | When |
| --- | --- | --- |
| 10 | VotingClosed | `vote` at or after `closes_at` |
| 11 | VotingOpen | `finalize` before `closes_at` |
| 12 | AlreadyVoted | Duplicate vote |
| 13 | AlreadyFinalized | Operation on closed proposal |
| 14 | SchemaNotWeighted | Schema weight is zero |
| 15 | CredentialNotValid | Verifier returned invalid |
| 16 | InvalidVotingPeriod | Period outside allowed range |
| 17 | WeightOverflow | Checked tally overflow |

## Events

- `SchemaWeightSet { schema_id, weight }`
- `ProposalCreated { proposal_id, proposer, closes_at }`
- `VoteCast { proposal_id, voter, weight, support }`
- `ProposalFinalized { proposal_id, yes_weight, no_weight, state }`

## Testing

Unit tests use a real `vc-verifier` instance wired to issuer, schema, and revocation registries (not stubbed verification).

```bash
cargo test -p credential-gated-dao-contract
```

Property tests cover voting invariants; fuzz target:

```bash
cd contracts/credential-gated-dao
cargo +nightly fuzz run fuzz_proposal_voting --sanitizer none -- -max_total_time=60
```

## Constants

- `MIN_VOTING_PERIOD = 3_600` (1 hour)
- `MAX_VOTING_PERIOD = 2_592_000` (30 days)
