#!/usr/bin/env sh

rustup target add wasm32-unknown-unknown
cargo install devserver
if command -v npm &> /dev/null
then
    cd ../tools/decode
    npm i
fi
