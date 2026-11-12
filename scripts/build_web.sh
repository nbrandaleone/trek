#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

cargo build --lib --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/trek.wasm web/trek.wasm
echo "Built web/trek.wasm ($(wc -c < web/trek.wasm | tr -d ' ') bytes)"
