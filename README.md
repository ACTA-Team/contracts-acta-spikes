# contracts-acta-spikes

Experimental implementations and explorations for smart contracts on Stellar.

This repository contains various contract implementations, proof-of-concepts, and research work exploring different approaches to on-chain systems.

## Structure

- `contracts/` - Individual contract implementations
  - `vc-issuer-registry` - Allowlist and metadata registry for VC issuers
  - `vc-revocation-registry` - Revocation status tracking for credentials
  - `vc-schema-registry` - On-chain registry for Verifiable Credential schema definitions
  - `credential-gated-dao` - Credential-weighted governance with proposal lifecycle
- `docs/` - Documentation and specifications
- `scripts/` - Build and deployment utilities

## Contracts

| Contract | Description |
| -------- | ----------- |
| [`vc-issuer-registry`](contracts/vc-issuer-registry/README.md) | On-chain allowlist and metadata registry for VC issuers |
| [`vc-revocation-registry`](contracts/vc-revocation-registry/README.md) | Revocation status tracking for credentials |
| [`vc-schema-registry`](contracts/vc-schema-registry/README.md) | On-chain registry for Verifiable Credential schema definitions |
| [`credential-gated-dao`](contracts/credential-gated-dao/README.md) | Credential-weighted governance with bounded proposal voting |

## Building

Build a single contract or all contracts in the workspace:

```bash
./scripts/build.sh vc-issuer-registry
./scripts/build.sh                # build all contracts
```

The script derives the list of contracts from `contracts/*/`, so adding a new
contract directory requires no script changes.

**Target triple**: `wasm32v1-none` (soroban-sdk 27 does not support
`wasm32-unknown-unknown` on Rust 1.82+)

## Deploying

```bash
./scripts/deploy.sh <contract> <network> <source-account>
```

Examples:

```bash
./scripts/deploy.sh vc-issuer-registry testnet acta_deployer
./scripts/deploy.sh vc-schema-registry testnet acta_deployer
```

The `CONTRACT_ADMIN` environment variable can be used to override the admin
address for contract initialization (defaults to the source account address).

## Release

Build and deploy all contracts to testnet:

```bash
./scripts/release.sh
```

Or deploy a specific contract:

```bash
./scripts/release.sh vc-issuer-registry
```

## Testing

```bash
cargo test
```

## License

Licensed under the [MIT License](./LICENSE).

