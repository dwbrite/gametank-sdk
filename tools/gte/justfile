#set shell := ["cmd.exe", "/c"]

build-wasm:
    cargo build --release --target wasm32-unknown-unknown
    wasm-bindgen \
    --out-dir web/bin \
    --target web \
    target/wasm32-unknown-unknown/release/gametank-emu-rs.wasm

run-wasm: build-wasm
    miniserve web --port 8080 --index index.html
