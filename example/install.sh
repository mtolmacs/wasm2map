#!/usr/bin/env sh

rustup target add wasm32-unknown-unknown
cargo install devserver
if command -v npm &> /dev/null
then
    cd ../tools/decode
    npm i
    cd ../tools/source-map
    npm i
fi
if command -v python3 &> /dev/null
then
    python3 -m pip install --user virtualenv
    cd ../tools/wasm-sourcemap
    python3 -m venv .env
fi
