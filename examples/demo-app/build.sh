#!/usr/bin/env bash
set -euo pipefail

# Build the demo app to WASM targeting WASI Preview 1.
# Requires: rustup target add wasm32-wasip1

cd "$(dirname "$0")"

echo "Building demo-app -> demo.wasm"
rustc --target wasm32-wasip1 \
    --edition 2021 \
    -o demo.wasm \
    src/main.rs

# Optional: strip with wasm-tools if available
if command -v wasm-tools &>/dev/null; then
    echo "Stripping with wasm-tools..."
    wasm-tools strip demo.wasm -o demo.wasm
fi

SIZE=$(wc -c < demo.wasm | tr -d ' ')
echo "Done: demo.wasm (${SIZE} bytes)"
