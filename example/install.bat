@ECHO OFF
rustup target add wasm32-unknown-unknown
cargo install simple-http-server
npm i typescript && cd ../tools/decode && npm i && cd ../source-map && npm i
