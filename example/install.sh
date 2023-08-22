#!/usr/bin/env sh

rustup target add wasm32-unknown-unknown
cargo install simple-http-server
npm i typescript
cd ../tools/decode
npm i
cd ../source-map
npm i
if command -v python3 &> /dev/null
then
    python3 -m pip install --user virtualenv
    cd ../wasm-sourcemap
    python3 -m venv .env
fi
