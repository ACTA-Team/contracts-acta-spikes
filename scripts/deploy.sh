#!/bin/sh
set -eu

# Deploy a contract to a Stellar network.
#
# Usage:
#   ./scripts/deploy.sh <contract> <network> <source-account>
#
#   contract:       one of the contracts in contracts/
#   network:        testnet | mainnet
#   source-account: stellar keys alias (e.g. acta_deployer)
#
# Examples:
#   ./scripts/deploy.sh vc-issuer-registry testnet acta_deployer
#   ./scripts/deploy.sh vc-schema-registry testnet acta_deployer
#
# Prerequisites:
#   - stellar-cli installed and configured
#   - Network already added:
#       stellar config network add testnet \
#         --rpc-url https://soroban-testnet.stellar.org:443 \
#         --network-passphrase "Test SDF Network ; September 2015"
#   - Source account key generated:
#       stellar keys generate acta_deployer --network <network>
#   - WASM built:
#       ./scripts/build.sh <contract>
#
# Environment variables:
#   CONTRACT_ADMIN  admin address for contract initialization (optional,
#                   defaults to source account address)

CONTRACT=${1:-}
NETWORK=${2:-}
SOURCE=${3:-}

SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONTRACTS_DIR="$ROOT_DIR/contracts"
WASM_TARGET=wasm32v1-none

if [ -z "$CONTRACT" ] || [ -z "$NETWORK" ] || [ -z "$SOURCE" ]; then
    # Build the list of available contracts from contracts/*/
    available=""
    if [ -d "$CONTRACTS_DIR" ]; then
        for dir in "$CONTRACTS_DIR"/*/; do
            [ -d "$dir" ] || continue
            name="$(basename "$dir")"
            available="$available $name"
        done
    fi
    echo "Usage: $0 <contract> <network> <source-account>" >&2
    echo "  contract:  one of:$available" >&2
    echo "  network:   testnet | mainnet" >&2
    exit 1
fi

# Validate the contract exists in contracts/
contract_dir="$CONTRACTS_DIR/$CONTRACT"
if [ ! -d "$contract_dir" ]; then
    echo "Unknown contract: $CONTRACT" >&2
    echo "Available contracts:" >&2
    if [ -d "$CONTRACTS_DIR" ]; then
        for dir in "$CONTRACTS_DIR"/*/; do
            [ -d "$dir" ] || continue
            echo "  $(basename "$dir")" >&2
        done
    fi
    exit 1
fi

cargo_toml="$contract_dir/Cargo.toml"
pkg_name="$(grep '^name =' "$cargo_toml" | head -1 | sed 's/^name = "\(.*\)"$/\1/')"
wasm_name="${pkg_name//-/_}.wasm"
WASM="target/${WASM_TARGET}/release/${wasm_name}"

ADMIN="${CONTRACT_ADMIN:-$(stellar keys address "$SOURCE")}"
CONSTRUCTOR_ARGS="-- --admin $ADMIN"

if [ ! -f "$WASM" ]; then
    echo "WASM not found: $WASM" >&2
    echo "Run: ./scripts/build.sh $CONTRACT" >&2
    exit 1
fi

echo "Deploying $CONTRACT to $NETWORK..."
echo "  WASM: $WASM"
echo "  Source: $SOURCE"
echo "  Admin: $ADMIN"

CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    $CONSTRUCTOR_ARGS)

echo ""
echo "Contract ID: $CONTRACT_ID"
echo ""
echo "Add this entry to docs/deployments/$NETWORK.md:"
echo ""
echo "| $CONTRACT | $(date +%Y-%m-%d) | \`$CONTRACT_ID\` | $NETWORK |"