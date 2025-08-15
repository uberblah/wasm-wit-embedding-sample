#!/bin/bash

set -e
set -x


# mkdir -p "guest_api"
# wit-bindgen rust ./wit/shellbound.wit --world program --out-dir "guest_api"

pushd sb_harness
cargo build --release
ln -sf ../wit
popd

for d in $(ls programs); do
    echo "Checking program directory entry: $d"
    if [ -d "programs/${d}" ]; then
        echo "Building program: $d"
        cargo build -p "${d}" --release --target wasm32-unknown-unknown
        mkdir -p "program_bins"
        wasm-tools component new "target/wasm32-unknown-unknown/release/${d}.wasm" --output "program_bins/${d}.wasm"
    fi
done