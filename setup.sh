#!/bin/bash

# you must first have rustup installed
# https://www.rust-lang.org/tools/install

rustup target add wasm32-unknown-unknown
cargo install wasm-tools