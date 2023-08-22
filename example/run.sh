#!/usr/bin/env sh

cargo clean
cargo build --target wasm32-unknown-unknown
cp src/index.html target/wasm32-unknown-unknown/debug/
cp src/index.ts target/wasm32-unknown-unknown/debug/
cp src/util.ts target/wasm32-unknown-unknown/debug/
touch target/wasm32-unknown-unknown/debug/favicon.ico
cargo install --path ../cargo-wasm2map
cargo wasm2map --bundle-sources -p -b http://127.0.0.1:8080 target/wasm32-unknown-unknown/debug/example.wasm
node ../tools/decode/decode.js ./target/wasm32-unknown-unknown/debug/example.wasm.map ./target/wasm32-unknown-unknown/debug/sourcemap.json || true
node_modules/.bin/tsc --sourcemap -m es6 target/wasm32-unknown-unknown/debug/index.ts
node_modules/.bin/tsc --sourcemap -m es6 target/wasm32-unknown-unknown/debug/util.ts
simple-http-server -p 8080 -i target/wasm32-unknown-unknown/debug/