#!/bin/sh
# Build the engine to WebAssembly and copy it next to index.html.
set -e
cd "$(dirname "$0")/wasm"
if command -v rustup >/dev/null && ! rustup target list --installed | grep -q '^wasm32-unknown-unknown$'; then
    echo "missing Rust target: run  rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/monopd_wasm.wasm ../monopd.wasm
echo "built web/monopd.wasm ($(wc -c < ../monopd.wasm) bytes)"
