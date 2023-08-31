@ECHO OFF
rustup target add wasm32-unknown-unknown
cargo install devserver
npm i typescript
cd ../tools/decode
npm i
cd ../source-map
npm i