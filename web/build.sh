#!/bin/sh
# Build the engine to WebAssembly and copy it next to index.html.
set -e
cd "$(dirname "$0")/wasm"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/monopd_wasm.wasm ../monopd.wasm
echo "built web/monopd.wasm ($(wc -c < ../monopd.wasm) bytes)"
