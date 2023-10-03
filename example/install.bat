@ECHO OFF
rustup target add wasm32-unknown-unknown
cargo install simple-http-server
npm i typescript && cd ../tools/decode && npm i && cd ../source-map && npm i && cd ../wasm-sourcemap && python3 -m venv .env && .\.env\Scripts\pip3.exe install -r requirements.txt && cd ../../example
