#!/usr/bin/env bash
# Build the in-browser GUI: compile ferroplan-bevy to wasm32 + generate JS glue.
# Prereqs: rustup target add wasm32-unknown-unknown; cargo install wasm-bindgen-cli.
set -euo pipefail
cd "$(dirname "$0")/../.."
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
  cargo build -p ferroplan-bevy --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript \
  --out-dir crates/ferroplan-bevy/web/pkg \
  target/wasm32-unknown-unknown/release/ferroplan-bevy.wasm
echo "Built. Serve: python3 -m http.server -d crates/ferroplan-bevy/web 8000"
