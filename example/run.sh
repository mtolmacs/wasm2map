#!/usr/bin/env sh

cargo clean
cargo build --target wasm32-unknown-unknown
cp src/index.html target/wasm32-unknown-unknown/debug/
touch target/wasm32-unknown-unknown/debug/favicon.ico
cargo install --path ../cargo-wasm2map
cargo wasm2map --bundle-sources -p -b http://127.0.0.1:8080 target/wasm32-unknown-unknown/debug/example.wasm
if command -v node &> /dev/null
then
    node ../tools/decode/decode.js ./target/wasm32-unknown-unknown/debug/example.wasm.map ./target/wasm32-unknown-unknown/debug/sourcemap.json || true
fi
devserver --noreload --path target/wasm32-unknown-unknown/debug/ --address 127.0.0.1:8080
