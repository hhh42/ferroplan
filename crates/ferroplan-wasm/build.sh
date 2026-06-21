#!/usr/bin/env bash
# Build the ferroplan WASM demo: compile the lib to wasm32, then generate the JS
# glue + module into web/pkg/. Prereqs:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli   # version must match the wasm-bindgen crate
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo build -p ferroplan-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir crates/ferroplan-wasm/web/pkg \
  target/wasm32-unknown-unknown/release/ferroplan_wasm.wasm
echo "Built. Serve with:  python3 -m http.server -d crates/ferroplan-wasm/web 8000"
