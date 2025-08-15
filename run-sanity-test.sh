#!/bin/bash

set -e
set -x

./build-programs.sh
pushd sb_cli
cargo run --release -- --wasm ../program_bins/sb_hello.wasm
popd