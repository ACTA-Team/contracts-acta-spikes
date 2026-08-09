#!/bin/sh
set -eu

# Build and deploy contracts to a Stellar network (testnet by default).
#
# Usage:
#   ./scripts/release.sh                  # build and deploy all contracts
#   ./scripts/release.sh vc-issuer-registry  # build and deploy one contract
#
# Environment variables:
#   DEPLOYER  stellar keys alias (default: vc_deployer)

CONTRACT=${1:-}
SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONTRACTS_DIR="$ROOT_DIR/contracts"
DEPLOYER=${DEPLOYER:-vc_deployer}

# Config testnet in local (idempotent: skip if already configured).
stellar config network ls 2>/dev/null | grep -q testnet || \
  stellar config network add testnet \
    --rpc-url https://soroban-testnet.stellar.org:443 \
    --network-passphrase "Test SDF Network ; September 2015"

# Generate key to sign the transactions (idempotent: skip if key already exists).
stellar keys show "$DEPLOYER" 2>/dev/null || \
  stellar keys generate "$DEPLOYER" --network testnet

# Build phase.
if [ -z "$CONTRACT" ]; then
    sh "$SCRIPT_DIR/build.sh"
else
    sh "$SCRIPT_DIR/build.sh" "$CONTRACT"
fi

# Deploy phase.
if [ -z "$CONTRACT" ]; then
    for dir in "$CONTRACTS_DIR"/*/; do
        [ -d "$dir" ] || continue
        name="$(basename "$dir")"
        sh "$SCRIPT_DIR/deploy.sh" "$name" testnet "$DEPLOYER"
    done
else
    sh "$SCRIPT_DIR/deploy.sh" "$CONTRACT" testnet "$DEPLOYER"
fi