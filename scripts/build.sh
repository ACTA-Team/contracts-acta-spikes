#!/bin/sh
set -eu

# Build and optimize WASM artifacts for one or all contracts.
#
# Usage:
#   ./scripts/build.sh                  # build all contracts
#   ./scripts/build.sh vc-issuer-registry  # build a specific contract
#
# Output: target/wasm32-unknown-unknown/release/<name>.optimized.wasm
#
# Target triple: wasm32-unknown-unknown (settled in place of wasm32v1-none)

WASM_TARGET=wasm32-unknown-unknown

SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONTRACTS_DIR="$ROOT_DIR/contracts"

# Collect contract directories: contracts/*/
contract_dirs=""
if [ -d "$CONTRACTS_DIR" ]; then
    for dir in "$CONTRACTS_DIR"/*/; do
        [ -d "$dir" ] || continue
        name="$(basename "$dir")"
        contract_dirs="$contract_dirs $name"
    done
fi

# Fallback: derive package names from contracts/*/Cargo.toml
get_package_name() {
    contract_dir="$1"
    cargo_toml="$CONTRACTS_DIR/$contract_dir/Cargo.toml"
    if [ -f "$cargo_toml" ]; then
        grep '^name =' "$cargo_toml" | head -1 | sed 's/^name = "\(.*\)"$/\1/'
    fi
}

# Build a single contract.
# Arguments: <contract-dir-name>
build_contract() {
    contract="$1"
    cargo_toml="$CONTRACTS_DIR/$contract/Cargo.toml"

    if [ ! -f "$cargo_toml" ]; then
        echo "No Cargo.toml found for: $contract" >&2
        exit 1
    fi

    pkg_name="$(get_package_name "$contract")"
    if [ -z "$pkg_name" ]; then
        echo "Could not determine package name for: $contract" >&2
        exit 1
    fi

    echo "Building: $contract ($pkg_name)"

    # Check if Cargo.toml declares cdylib. If not, override via cargo rustc.
    has_cdylib="$(grep -c 'crate-type.*cdylib' "$cargo_toml" || true)"

    if [ "$has_cdylib" -gt 0 ]; then
        cargo build \
            -p "$pkg_name" \
            --target "$WASM_TARGET" \
            --release
    else
        cargo rustc \
            -p "$pkg_name" \
            --target "$WASM_TARGET" \
            --release \
            -- --crate-type cdylib
    fi

    # Derive the WASM filename from the package name (dashes → underscores).
    wasm_name="${pkg_name//-/_}.wasm"
    wasm_path="target/${WASM_TARGET}/release/${wasm_name}"

    if [ ! -f "$wasm_path" ]; then
        echo "WASM not found after build: $wasm_path" >&2
        exit 1
    fi

    stellar contract optimize --wasm "$wasm_path"
    echo "Built: ${wasm_path%.wasm}.optimized.wasm"
}

PACKAGE=${1:-all}

case "$PACKAGE" in
    all)
        if [ -z "$contract_dirs" ]; then
            echo "No contracts found in contracts/" >&2
            exit 1
        fi
        for dir in $contract_dirs; do
            build_contract "$dir"
        done
        ;;
    *)
        found=0
        for dir in $contract_dirs; do
            if [ "$dir" = "$PACKAGE" ]; then
                found=1
                break
            fi
        done
        if [ "$found" -eq 0 ]; then
            echo "Unknown package: $PACKAGE" >&2
            echo "Available packages:" >&2
            for dir in $contract_dirs; do
                echo "  $dir" >&2
            done
            exit 1
        fi
        build_contract "$PACKAGE"
        ;;
esac