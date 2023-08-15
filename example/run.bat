@ECHO OFF
cargo clean
cargo build --target wasm32-unknown-unknown
COPY src\index.html target\wasm32-unknown-unknown\debug\
copy nul >> target\wasm32-unknown-unknown\debug\favicon.ico
cargo install --path ..\cargo-wasm2map
cargo wasm2map -p -b http://127.0.0.1:8080 --bundle-sources target\wasm32-unknown-unknown\debug\example.wasm
node ..\tools\decode\decode.js .\target\wasm32-unknown-unknown\debug\example.wasm.map .\target\wasm32-unknown-unknown\debug\sourcemap.json
devserver --noreload --path target\wasm32-unknown-unknown\debug\ --address 127.0.0.1:8080
