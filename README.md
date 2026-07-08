# contracts-acta-spikes

Experimental implementations and explorations for smart contracts on Stellar.

This repository contains various contract implementations, proof-of-concepts, and research work exploring different approaches to on-chain systems.

## Structure

- `contracts/` - Individual contract implementations
  - `vc-issuer-registry` - Allowlist and metadata registry for VC issuers
  - `vc-revocation-registry` - Revocation status tracking for credentials
- `docs/` - Documentation and specifications
- `scripts/` - Build and deployment utilities

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
