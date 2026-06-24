# contracts-acta-spikes

Experimental implementations and explorations for smart contracts on Stellar.

This repository contains various contract implementations, proof-of-concepts, and research work exploring different approaches to on-chain systems.

## Structure

- `contracts/` - Individual contract implementations
- `docs/` - Documentation and specifications
- `scripts/` - Build and deployment utilities

## Contracts

| Contract | Description |
| -------- | ----------- |
| [`vc-issuer-registry`](contracts/vc-issuer-registry/README.md) | On-chain allowlist and metadata registry for VC issuers |
| [`vc-schema-registry`](contracts/vc-schema-registry/README.md) | On-chain registry for Verifiable Credential schema definitions |

## Building

```bash
./scripts/build.sh <contract-name>
```

## Testing

```bash
cargo test
```

## License

Licensed under the [MIT License](./LICENSE).
